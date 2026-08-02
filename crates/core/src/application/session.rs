use std::path::PathBuf;

use crate::application::add_sources::AddSources;
use crate::application::ports::{ImageDecoder, PdfEngine};
use crate::domain::document::MergeDocument;
use crate::domain::ids::{IdSequence, SlotId, SourceId};
use crate::domain::operations;
use crate::domain::plan::{MergePlan, SlotRange};
use crate::domain::source::SourceFile;

const HISTORY_LIMIT: usize = 100;

#[derive(Debug)]
pub struct PlanSession {
    document: MergeDocument,
    // IDs are deliberately not snapshotted so they remain monotonic across undo.
    ids: IdSequence,
    undo: Vec<MergeDocument>,
    redo: Vec<MergeDocument>,
}

impl PlanSession {
    pub fn new() -> Self {
        Self {
            document: MergeDocument::default(),
            ids: IdSequence::default(),
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn plan(&self) -> &MergePlan {
        self.document.plan()
    }

    pub fn sources(&self) -> &[SourceFile] {
        self.document.sources()
    }

    pub fn document(&self) -> &MergeDocument {
        &self.document
    }

    /// Whether the source's pages currently form one ascending run, and so are
    /// drawn as a single card. Derived from the plan on every call: the plan is
    /// the only thing that decides it.
    pub fn is_grouped(&self, source: SourceId) -> bool {
        self.document.is_grouped(source)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn add_sources(
        &mut self,
        pdf: &dyn PdfEngine,
        images: &dyn ImageDecoder,
        paths: &[PathBuf],
    ) {
        self.begin_change();
        self.document = AddSources { pdf, images }.execute(&self.document, &mut self.ids, paths);
        self.finish_change();
    }

    pub fn reorder(&mut self, from_start: usize, from_end: usize, to: usize) {
        let Some(from) = SlotRange::resolve(self.document.plan(), from_start, from_end) else {
            return;
        };
        self.begin_change();
        let plan = operations::reorder(self.document.plan(), from, to);
        self.document = self.document.with_plan(plan);
        self.finish_change();
    }

    pub fn remove(&mut self, ids: &[SlotId]) {
        self.begin_change();
        let plan = operations::remove(self.document.plan(), ids);
        self.document = self.document.with_plan(plan);
        self.finish_change();
    }

    /// Removes a source and every slot it owns. This is the only way to take a
    /// file out of the document by name rather than by page, and the only way
    /// at all to dismiss one that contributes no pages: a failed source has no
    /// slot to select, so `remove` cannot reach it.
    pub fn remove_source(&mut self, source: SourceId) {
        self.begin_change();
        let ids: Vec<SlotId> = self
            .document
            .plan()
            .slots()
            .iter()
            .filter(|slot| slot.source == source)
            .map(|slot| slot.id)
            .collect();
        let plan = operations::remove(self.document.plan(), &ids);
        let sources = self
            .document
            .sources()
            .iter()
            .filter(|candidate| candidate.id() != source)
            .cloned()
            .collect();
        self.document = MergeDocument::new(plan, sources);
        self.finish_change();
    }

    pub fn rotate(&mut self, ids: &[SlotId], delta: i32) {
        self.begin_change();
        let plan = operations::rotate(self.document.plan(), ids, delta);
        self.document = self.document.with_plan(plan);
        self.finish_change();
    }

    pub fn undo(&mut self) -> bool {
        let Some(snapshot) = self.undo.pop() else {
            return false;
        };
        let current = self.snapshot();
        push_bounded(&mut self.redo, current);
        self.install(snapshot);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(snapshot) = self.redo.pop() else {
            return false;
        };
        let current = self.snapshot();
        push_bounded(&mut self.undo, current);
        self.install(snapshot);
        true
    }

    /// Parks the current state on the undo stack for `finish_change` to either
    /// keep or discard. Nothing is cleared here: whether the command turns out
    /// to be a no-op is not known yet, and a no-op must leave redo alone.
    fn begin_change(&mut self) {
        let current = self.snapshot();
        self.undo.push(current);
    }

    /// Keeps the parked entry only if the command actually moved something. A
    /// command that changed nothing -- a Delete against a stale selection, a
    /// degenerate drag range -- must not arm Undo, and must not cost the user
    /// their redo.
    fn finish_change(&mut self) {
        let previous = self
            .undo
            .pop()
            .expect("begin_change must precede finish_change");
        self.document = self.document.dropping_sources_emptied_since(&previous);

        // Sources are compared too: adding an encrypted or unreadable file
        // appends a zero-page source without contributing a slot, and undoing
        // that has to take its error card away again.
        if self.document == previous {
            return;
        }

        push_bounded(&mut self.undo, previous);
        self.redo.clear();
    }

    fn snapshot(&self) -> MergeDocument {
        self.document.clone()
    }

    fn install(&mut self, snapshot: MergeDocument) {
        self.document = snapshot;
    }
}

impl Default for PlanSession {
    fn default() -> Self {
        Self::new()
    }
}

fn push_bounded(stack: &mut Vec<MergeDocument>, snapshot: MergeDocument) {
    // `>=` rather than `==`: `begin_change` pushes unbounded so that
    // `finish_change` can take the entry back, so a command that panics in
    // between leaves the stack one past the limit rather than exactly on it.
    while stack.len() >= HISTORY_LIMIT {
        stack.remove(0);
    }
    stack.push(snapshot);
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::application::errors::PdfError;
    use crate::domain::geometry::PageSize;
    use crate::domain::ids::SlotId;
    use crate::domain::source::{DocumentInfo, ImageInfo, SourceStatus};
    use crate::infrastructure::fake_engine::{FakeImageDecoder, FakePdfEngine};

    fn document(page_count: u32) -> DocumentInfo {
        DocumentInfo {
            page_count,
            page_sizes: vec![PageSize::A4_PORTRAIT; page_count as usize],
            encrypted: false,
        }
    }

    fn session_with_pdf(page_count: u32) -> PlanSession {
        let pdf = FakePdfEngine::new().with_document("/document.pdf", document(page_count));
        let mut session = PlanSession::new();
        session.add_sources(&pdf, &FakeImageDecoder::new(), &["/document.pdf".into()]);
        session
    }

    fn apply_arbitrary_op(session: &mut PlanSession, op: usize) {
        match op {
            0 => {
                let len = session.plan().len();
                session.reorder(len.saturating_sub(1), len, len / 2);
            }
            1 => {
                let len = session.plan().len();
                session.reorder(0, usize::from(len > 0), len.saturating_sub(1));
            }
            2 => {
                let ids = session
                    .plan()
                    .slots()
                    .first()
                    .map(|slot| vec![slot.id])
                    .unwrap_or_default();
                session.remove(&ids);
            }
            _ => unreachable!("the strategy only produces operations 0 through 2"),
        }
    }

    #[test]
    fn undo_restores_the_previous_plan() {
        let mut s = session_with_pdf(3);
        let before = s.plan().clone();
        s.remove(&[s.plan().slots()[0].id]);
        assert_eq!(s.plan().len(), 2);
        assert!(s.undo());
        assert_eq!(s.plan(), &before);
    }

    #[test]
    fn redo_reapplies_an_undone_change() {
        let mut s = session_with_pdf(3);
        s.remove(&[s.plan().slots()[0].id]);
        let after_remove = s.plan().clone();
        s.undo();
        assert!(s.redo());
        assert_eq!(s.plan(), &after_remove);
    }

    #[test]
    fn a_new_change_clears_the_redo_stack() {
        let mut s = session_with_pdf(3);
        s.remove(&[s.plan().slots()[0].id]);
        s.undo();
        s.remove(&[s.plan().slots()[1].id]);
        assert!(!s.can_redo());
    }

    #[test]
    fn a_no_op_does_not_clear_the_redo_stack() {
        let mut s = session_with_pdf(3);
        s.undo.clear();
        s.remove(&[s.plan().slots()[0].id]);
        assert!(s.undo());

        s.remove(&[SlotId(9999)]);

        assert!(s.can_redo());
    }

    #[test]
    fn undo_restores_grouping_state_as_well_as_the_plan() {
        let mut s = session_with_pdf(3);
        let source_id = s.sources()[0].id();
        let images = FakeImageDecoder::new().with_image(
            "/image.png",
            ImageInfo {
                width_px: 640,
                height_px: 480,
            },
        );
        s.add_sources(&FakePdfEngine::new(), &images, &["/image.png".into()]);
        let image_index = s.plan().len() - 1;

        // The drag the UI can actually make: the trailing image slot lands
        // strictly inside the PDF's run, which is what ungroups the PDF.
        s.reorder(image_index, image_index + 1, 1);
        assert!(!s.is_grouped(source_id));
        s.undo();
        assert!(s.is_grouped(source_id));
    }

    #[test]
    fn undo_on_an_empty_history_is_a_no_op() {
        let mut s = PlanSession::new();
        assert!(!s.undo());
    }

    #[test]
    fn removing_every_slot_of_a_source_discards_the_source() {
        let mut s = session_with_pdf(2);
        let ids: Vec<SlotId> = s.plan().slots().iter().map(|x| x.id).collect();
        s.remove(&ids);
        assert!(s.sources().is_empty());
    }

    #[test]
    fn removing_an_encrypted_source_discards_it() {
        let pdf = FakePdfEngine::new().with_failure(
            "/locked.pdf",
            PdfError::Encrypted {
                path: "/locked.pdf".into(),
            },
        );
        let mut s = PlanSession::new();
        s.add_sources(&pdf, &FakeImageDecoder::new(), &["/locked.pdf".into()]);
        let source_id = s.sources()[0].id();

        s.remove_source(source_id);

        assert!(s.sources().is_empty());
    }

    #[test]
    fn removing_a_ready_source_discards_it_and_all_its_slots() {
        let mut s = session_with_pdf(2);
        let source_id = s.sources()[0].id();

        s.remove_source(source_id);

        assert!(s.sources().is_empty());
        assert!(s.plan().is_empty());
    }

    #[test]
    fn removing_an_unknown_source_does_not_create_history() {
        let mut s = session_with_pdf(2);
        s.undo.clear();

        s.remove_source(SourceId(9999));

        assert_eq!(s.sources().len(), 1);
        assert_eq!(s.plan().len(), 2);
        assert!(!s.can_undo());
    }

    #[test]
    fn removing_a_source_can_be_undone() {
        let mut s = session_with_pdf(2);
        let source = s.sources()[0].clone();
        let slots = s.plan().slots().to_vec();

        s.remove_source(source.id());
        assert!(s.undo());

        assert_eq!(s.sources(), &[source]);
        assert_eq!(s.plan().slots(), slots);
    }

    #[test]
    fn removing_one_source_preserves_the_order_of_the_other_sources_slots() {
        let pdf = FakePdfEngine::new()
            .with_document("/first.pdf", document(2))
            .with_document("/second.pdf", document(2));
        let mut s = PlanSession::new();
        s.add_sources(
            &pdf,
            &FakeImageDecoder::new(),
            &["/first.pdf".into(), "/second.pdf".into()],
        );
        let removed_source = s.sources()[0].id();
        let surviving_source = s.sources()[1].id();
        let surviving_slots = s
            .plan()
            .slots()
            .iter()
            .filter(|slot| slot.source == surviving_source)
            .cloned()
            .collect::<Vec<_>>();

        s.remove_source(removed_source);

        assert_eq!(s.sources().len(), 1);
        assert_eq!(s.sources()[0].id(), surviving_source);
        assert_eq!(s.plan().slots(), surviving_slots);
    }

    #[test]
    fn can_undo_and_can_redo_follow_history_transitions() {
        let mut s = PlanSession::new();
        assert!(!s.can_undo());
        assert!(!s.can_redo());

        let pdf = FakePdfEngine::new().with_document("/document.pdf", document(1));
        s.add_sources(&pdf, &FakeImageDecoder::new(), &["/document.pdf".into()]);
        assert!(s.can_undo());
        assert!(!s.can_redo());

        assert!(s.undo());
        assert!(!s.can_undo());
        assert!(s.can_redo());

        assert!(s.redo());
        assert!(s.can_undo());
        assert!(!s.can_redo());
    }

    #[test]
    fn removing_an_unknown_slot_does_not_create_history() {
        let mut s = session_with_pdf(3);
        s.undo.clear();

        s.remove(&[SlotId(9999)]);

        assert!(!s.can_undo());
    }

    #[test]
    fn degenerate_reorders_do_not_create_history() {
        let ranges = [(1, 1), (2, 1), (90, 99)];

        for (from_start, from_end) in ranges {
            let mut s = session_with_pdf(3);
            s.undo.clear();

            s.reorder(from_start, from_end, 0);

            assert!(!s.can_undo());
        }
    }

    #[test]
    fn a_reorder_that_names_no_slots_leaves_redo_alone() {
        let mut s = session_with_pdf(3);
        s.remove(&[s.plan().slots()[0].id]);
        assert!(s.undo());

        s.reorder(1, 1, 0);

        assert!(s.can_redo());
    }

    #[test]
    fn the_cap_still_holds_after_a_command_panics_mid_change() {
        // `begin_change` pushes unbounded so that `finish_change` can drop the
        // entry again; a command that panics in between leaves that entry
        // behind, taking the stack one past the limit. The cap has to recover.
        let mut s = session_with_pdf(3);
        s.undo.clear();
        for _ in 0..=HISTORY_LIMIT {
            s.undo.push(s.snapshot());
        }

        s.remove(&[s.plan().slots()[0].id]);

        assert_eq!(s.undo.len(), HISTORY_LIMIT);
    }

    #[test]
    fn history_is_capped_at_one_hundred_entries() {
        let mut s = session_with_pdf(3);
        for _ in 0..125 {
            s.reorder(0, 1, 2);
        }

        // Only the newest hundred states stay reachable; the rest are dropped.
        let mut undone = 0;
        while s.undo() {
            undone += 1;
        }
        assert_eq!(undone, HISTORY_LIMIT);
        assert_eq!(s.plan().len(), 3);
        assert_eq!(s.sources().len(), 1);
    }

    #[test]
    fn an_encrypted_source_survives_later_mutations() {
        let pdf = FakePdfEngine::new()
            .with_document("/document.pdf", document(2))
            .with_failure(
                "/locked.pdf",
                PdfError::Encrypted {
                    path: "/locked.pdf".into(),
                },
            );
        let mut s = PlanSession::new();
        s.add_sources(
            &pdf,
            &FakeImageDecoder::new(),
            &["/document.pdf".into(), "/locked.pdf".into()],
        );

        s.remove(&[s.plan().slots()[0].id]);

        assert_eq!(s.sources().len(), 2);
        assert!(s
            .sources()
            .iter()
            .any(|source| source.status() == SourceStatus::Encrypted));
    }

    #[test]
    fn adding_an_encrypted_source_can_be_undone() {
        let pdf = FakePdfEngine::new().with_failure(
            "/locked.pdf",
            PdfError::Encrypted {
                path: "/locked.pdf".into(),
            },
        );
        let mut s = PlanSession::new();

        s.add_sources(&pdf, &FakeImageDecoder::new(), &["/locked.pdf".into()]);

        assert!(s.can_undo());
        assert_eq!(s.sources().len(), 1);
        assert_eq!(s.sources()[0].status(), SourceStatus::Encrypted);
        assert!(s.undo());
        assert!(s.sources().is_empty());
    }

    #[test]
    fn reorder_moves_a_range_in_the_session() {
        let mut s = session_with_pdf(4);
        let before: Vec<SlotId> = s.plan().slots().iter().map(|slot| slot.id).collect();

        s.reorder(0, 2, 2);

        let after: Vec<SlotId> = s.plan().slots().iter().map(|slot| slot.id).collect();
        assert_eq!(after, vec![before[2], before[3], before[0], before[1]]);
    }

    proptest! {
        /// undo followed by redo is the identity on both plan and sources.
        #[test]
        fn undo_then_redo_is_the_identity(ops in prop::collection::vec(0usize..3, 1..10)) {
            let mut s = session_with_pdf(5);
            for op in &ops { apply_arbitrary_op(&mut s, *op); }
            let snapshot = (s.plan().clone(), s.sources().to_vec());
            if s.undo() { prop_assert!(s.redo()); }
            prop_assert_eq!((s.plan().clone(), s.sources().to_vec()), snapshot);
        }
    }
}
