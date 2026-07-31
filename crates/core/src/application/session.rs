use std::ops::Range;
use std::path::PathBuf;

use crate::application::add_sources::AddSources;
use crate::application::ports::{ImageDecoder, PdfEngine};
use crate::domain::grouping::can_regroup;
use crate::domain::ids::{IdSequence, SlotId, SourceId};
use crate::domain::operations;
use crate::domain::plan::MergePlan;
use crate::domain::source::SourceFile;

const HISTORY_LIMIT: usize = 100;

#[derive(Debug)]
struct Snapshot {
    plan: MergePlan,
    sources: Vec<SourceFile>,
}

#[derive(Debug)]
pub struct PlanSession {
    plan: MergePlan,
    sources: Vec<SourceFile>,
    // IDs are deliberately not snapshotted so they remain monotonic across undo.
    ids: IdSequence,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
}

impl PlanSession {
    pub fn new() -> Self {
        Self {
            plan: MergePlan::default(),
            sources: Vec::new(),
            ids: IdSequence::default(),
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn plan(&self) -> &MergePlan {
        &self.plan
    }

    pub fn sources(&self) -> &[SourceFile] {
        &self.sources
    }

    /// Whether the source's pages currently form one ascending run, and so are
    /// drawn as a single card. Derived from the plan on every call: the plan is
    /// the only thing that decides it.
    pub fn is_grouped(&self, source: SourceId) -> bool {
        can_regroup(&self.plan, source)
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
        let previous_plan = self.begin_change();
        let result =
            AddSources { pdf, images }.execute(&self.plan, &self.sources, &mut self.ids, paths);
        self.plan = result.plan;
        self.sources = result.sources;
        self.finish_change(&previous_plan);
    }

    /// Moves the named existing slots to `at`, indexed into the sequence that
    /// remains after those slots are removed. Unknown IDs are ignored.
    pub fn insert_at(&mut self, at: usize, slot_ids: &[SlotId]) {
        let previous_plan = self.begin_change();
        let slots = self
            .plan
            .slots()
            .iter()
            .filter(|slot| slot_ids.contains(&slot.id))
            .cloned()
            .collect::<Vec<_>>();
        let remaining = operations::remove(&self.plan, slot_ids);
        self.plan = operations::insert_at(&remaining, at, &slots);
        self.finish_change(&previous_plan);
    }

    pub fn reorder(&mut self, from: Range<usize>, to: usize) {
        let previous_plan = self.begin_change();
        self.plan = operations::reorder(&self.plan, from, to);
        self.finish_change(&previous_plan);
    }

    pub fn remove(&mut self, ids: &[SlotId]) {
        let previous_plan = self.begin_change();
        self.plan = operations::remove(&self.plan, ids);
        self.finish_change(&previous_plan);
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

    fn begin_change(&mut self) -> MergePlan {
        let previous_plan = self.plan.clone();
        let current = self.snapshot();
        push_bounded(&mut self.undo, current);
        self.redo.clear();
        previous_plan
    }

    fn finish_change(&mut self, previous_plan: &MergePlan) {
        self.sources.retain(|source| {
            let owned_before = previous_plan
                .slots()
                .iter()
                .any(|slot| slot.source == source.id);
            let owns_now = self
                .plan
                .slots()
                .iter()
                .any(|slot| slot.source == source.id);

            // A source that never owned a slot represents an encrypted or
            // unreadable file that the UI must continue to show.
            !owned_before || owns_now
        });
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            plan: self.plan.clone(),
            sources: self.sources.clone(),
        }
    }

    fn install(&mut self, snapshot: Snapshot) {
        self.plan = snapshot.plan;
        self.sources = snapshot.sources;
    }
}

impl Default for PlanSession {
    fn default() -> Self {
        Self::new()
    }
}

fn push_bounded(stack: &mut Vec<Snapshot>, snapshot: Snapshot) {
    if stack.len() == HISTORY_LIMIT {
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
                let ids = session
                    .plan()
                    .slots()
                    .last()
                    .map(|slot| vec![slot.id])
                    .unwrap_or_default();
                session.insert_at(session.plan().len() / 2, &ids);
            }
            1 => {
                let len = session.plan().len();
                session.reorder(0..usize::from(len > 0), len.saturating_sub(1));
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
    fn undo_restores_grouping_state_as_well_as_the_plan() {
        let mut s = session_with_pdf(3);
        let source_id = s.sources()[0].id;
        let images = FakeImageDecoder::new().with_image(
            "/image.png",
            ImageInfo {
                width_px: 640,
                height_px: 480,
            },
        );
        s.add_sources(&FakePdfEngine::new(), &images, &["/image.png".into()]);
        let image_slot = s.plan().slots().last().expect("image slot").id;

        s.insert_at(1, &[image_slot]);
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
    fn can_undo_and_can_redo_follow_history_transitions() {
        let mut s = PlanSession::new();
        assert!(!s.can_undo());
        assert!(!s.can_redo());

        s.remove(&[]);
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
    fn history_is_capped_at_one_hundred_entries() {
        let mut s = session_with_pdf(3);
        for _ in 0..125 {
            s.reorder(0..1, 2);
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
            .any(|source| source.status == SourceStatus::Encrypted));
    }

    #[test]
    fn reorder_moves_a_range_in_the_session() {
        let mut s = session_with_pdf(4);
        let before: Vec<SlotId> = s.plan().slots().iter().map(|slot| slot.id).collect();

        s.reorder(0..2, 2);

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
