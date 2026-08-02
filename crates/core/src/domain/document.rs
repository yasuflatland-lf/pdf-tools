use std::collections::HashMap;

use super::geometry::{PageSize, SizeClass};
use super::grouping::can_regroup;
use super::ids::SourceId;
use super::plan::{MergePlan, PageSlot};
use super::source::{SourceFile, SourceKind};

/// The document: an ordered plan together with the sources its slots name.
/// Constructing one is the only place the referential invariant is
/// established. Construction drops orphan slots rather than returning an
/// error so installing undo and redo snapshots remains infallible. Sources
/// without slots remain present because failed files are shown in the UI.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MergeDocument {
    plan: MergePlan,
    sources: Vec<SourceFile>,
}

impl MergeDocument {
    /// Creates a document, dropping any slot whose source is absent.
    pub fn new(plan: MergePlan, sources: Vec<SourceFile>) -> Self {
        let slots = plan
            .slots()
            .iter()
            .filter(|slot| sources.iter().any(|source| source.id() == slot.source))
            .cloned()
            .collect();

        Self {
            plan: MergePlan::new(slots),
            sources,
        }
    }

    pub fn plan(&self) -> &MergePlan {
        &self.plan
    }

    pub fn sources(&self) -> &[SourceFile] {
        &self.sources
    }

    pub(crate) fn with_plan(&self, plan: MergePlan) -> Self {
        Self::new(plan, self.sources.clone())
    }

    /// Returns the source named by a slot from this document's plan.
    ///
    /// This is total for slots obtained from `self.plan()` because construction
    /// establishes that every such slot names a listed source.
    pub fn source_of(&self, slot: &PageSlot) -> &SourceFile {
        self.sources
            .iter()
            .find(|source| source.id() == slot.source)
            .expect("MergeDocument construction guarantees every slot has a source")
    }

    /// The page size that images are fitted to: the most frequent size among
    /// the document's PDF-backed slots. Ties go to whichever size appears
    /// first in the plan. Falls back to A4 portrait when the plan contains no
    /// PDF pages. Sizes are classified on a one-point lattice.
    ///
    /// This is a **sheet** size, taken before any rotation. Composition writes
    /// the slot's rotation as the page's `/Rotate` attribute, for image-backed
    /// and PDF-backed slots alike, so turning a slot must not change what this
    /// returns -- otherwise the turn lands on an image page twice.
    pub fn dominant_page_size(&self) -> PageSize {
        let buckets = self
            .plan
            .slots()
            .iter()
            .enumerate()
            .filter_map(|(plan_index, slot)| {
                let source = self.source_of(slot);
                if source.kind() != SourceKind::Pdf {
                    return None;
                }
                source
                    .page_sizes()
                    .get(slot.page.0 as usize)
                    .copied()
                    .map(|page_size| (plan_index, page_size))
            })
            .fold(
                HashMap::<SizeClass, (usize, usize, PageSize)>::new(),
                |mut buckets, (plan_index, page_size)| {
                    buckets
                        .entry(page_size.size_class())
                        .and_modify(|(count, _, _)| *count += 1)
                        .or_insert((1, plan_index, page_size));
                    buckets
                },
            );

        buckets
            .into_values()
            .max_by(|left, right| left.0.cmp(&right.0).then_with(|| right.1.cmp(&left.1)))
            .map_or(PageSize::A4_PORTRAIT, |(_, _, representative)| {
                representative
            })
    }

    pub fn is_grouped(&self, source: SourceId) -> bool {
        can_regroup(&self.plan, source)
    }

    /// Returns this document with the sources the plan no longer justifies
    /// dropped.
    ///
    /// A source is kept when it still owns a slot, and also when it owned none
    /// in `previous`: that shape is an encrypted or unreadable file, which
    /// contributes no page and which the UI must continue to show. Losing the
    /// last slot is a statement about a transition rather than about a single
    /// document, which is why the document the operation started from is a
    /// parameter here.
    pub fn dropping_sources_emptied_since(&self, previous: &Self) -> Self {
        let sources = self
            .sources
            .iter()
            .filter(|source| !previous.owns_slot(source.id()) || self.owns_slot(source.id()))
            .cloned()
            .collect();

        Self::new(self.plan.clone(), sources)
    }

    /// Whether any slot in the plan names this source.
    fn owns_slot(&self, source: SourceId) -> bool {
        self.plan.slots().iter().any(|slot| slot.source == source)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use proptest::prelude::*;

    use super::*;
    use crate::domain::ids::{PageIndex, SlotId, SourceId};
    use crate::domain::operations;

    const A4: PageSize = PageSize::A4_PORTRAIT;

    fn letter() -> PageSize {
        PageSize::new(612.0, 792.0).expect("Letter page size should be valid")
    }

    fn plan_with(pages: &[(u64, u32)]) -> MergePlan {
        MergePlan::new(
            pages
                .iter()
                .enumerate()
                .map(|(slot_id, &(source_id, page_index))| PageSlot {
                    id: SlotId(slot_id as u64),
                    source: SourceId(source_id),
                    page: PageIndex(page_index),
                    rotation: Default::default(),
                })
                .collect(),
        )
    }

    fn pdf_source(id: u64, page_sizes: Vec<PageSize>) -> SourceFile {
        SourceFile::ready_pdf(SourceId(id), PathBuf::new(), page_sizes)
    }

    /// An image carries no page size at all, so `dominant_page_size` has
    /// nothing to count even before it filters image sources out by kind.
    fn image_source(id: u64) -> SourceFile {
        SourceFile::ready_image(SourceId(id), PathBuf::new())
    }

    fn document_with(pages: &[(u64, u32)], sources: Vec<SourceFile>) -> MergeDocument {
        MergeDocument::new(plan_with(pages), sources)
    }

    fn page_sizes_with_shuffle() -> impl Strategy<Value = (Vec<PageSize>, Vec<usize>)> {
        let page_size = || {
            (1.0f32..2000.0, 1.0f32..2000.0).prop_map(|(width_pt, height_pt)| {
                PageSize::new(width_pt, height_pt).expect("generated page size should be valid")
            })
        };

        (prop::collection::vec(page_size(), 0..20), page_size())
            .prop_flat_map(|(mut page_sizes, dominant)| {
                let background_len = page_sizes.len();
                page_sizes.extend(std::iter::repeat_n(dominant, background_len + 1));
                let len = page_sizes.len();
                (Just(page_sizes), prop::collection::vec(any::<u64>(), len))
            })
            .prop_map(|(page_sizes, keys)| {
                let mut shuffled_indices: Vec<usize> = (0..page_sizes.len()).collect();
                shuffled_indices.sort_by_key(|&index| (keys[index], index));
                (page_sizes, shuffled_indices)
            })
    }

    #[test]
    fn construction_drops_slots_whose_source_is_absent() {
        let orphan = PageSlot {
            id: SlotId(1),
            source: SourceId(99),
            page: PageIndex(0),
            rotation: Default::default(),
        };

        let document = MergeDocument::new(MergePlan::new(vec![orphan]), Vec::new());

        assert!(document.plan().is_empty());
    }

    #[test]
    fn source_of_is_total_for_a_slot_in_the_document() {
        let document = document_with(&[(10, 0)], vec![pdf_source(10, vec![A4])]);

        assert_eq!(
            document.source_of(&document.plan().slots()[0]).id(),
            SourceId(10)
        );
    }

    #[test]
    fn a_source_that_lost_its_last_slot_is_dropped() {
        let sources = vec![pdf_source(10, vec![A4])];
        let previous = document_with(&[(10, 0)], sources.clone());
        let current = document_with(&[], sources);

        assert!(current
            .dropping_sources_emptied_since(&previous)
            .sources()
            .is_empty());
    }

    #[test]
    fn a_source_that_never_owned_a_slot_is_kept() {
        let source = image_source(10);
        let sources = vec![source.clone()];
        let previous = document_with(&[], sources.clone());
        let current = document_with(&[], sources);

        assert_eq!(
            current.dropping_sources_emptied_since(&previous).sources(),
            &[source]
        );
    }

    #[test]
    fn a_source_that_still_owns_a_slot_is_kept() {
        let source = pdf_source(10, vec![A4, A4]);
        let sources = vec![source.clone()];
        let previous = document_with(&[(10, 0), (10, 1)], sources.clone());
        let current = document_with(&[(10, 1)], sources);

        assert_eq!(
            current.dropping_sources_emptied_since(&previous).sources(),
            &[source]
        );
    }

    #[test]
    fn the_most_frequent_pdf_page_size_wins() {
        // Two A4 pages, one Letter page.
        let document = document_with(
            &[(10, 0), (10, 1), (10, 2)],
            vec![pdf_source(10, vec![A4, A4, letter()])],
        );
        assert_eq!(document.dominant_page_size().size_class(), A4.size_class());
    }

    #[test]
    fn a_tie_is_broken_by_first_appearance_in_the_plan() {
        let document = document_with(
            &[(10, 0), (10, 1)],
            vec![pdf_source(10, vec![letter(), A4])],
        );
        assert_eq!(
            document.dominant_page_size().size_class(),
            letter().size_class()
        );
    }

    #[test]
    fn sizes_in_the_same_lattice_cell_are_counted_together() {
        let almost_a4 = PageSize::new(595.4, 841.6).expect("almost-A4 page size should be valid");
        let document = document_with(
            &[(10, 0), (10, 1), (10, 2)],
            vec![pdf_source(10, vec![A4, almost_a4, letter()])],
        );
        // A4 and almost-A4 form one bucket of 2, beating Letter's 1.
        assert_eq!(document.dominant_page_size().size_class(), A4.size_class());
    }

    #[test]
    fn a_lattice_cell_of_near_equal_sizes_outvotes_an_earlier_lone_size() {
        let almost_a4 = PageSize::new(595.4, 841.6).expect("almost-A4 page size should be valid");
        let document = document_with(
            &[(10, 0), (10, 1), (10, 2)],
            vec![pdf_source(10, vec![letter(), A4, almost_a4])],
        );
        // Letter appears first but stands alone; A4 and almost-A4 share a bucket.
        assert_eq!(document.dominant_page_size().size_class(), A4.size_class());
    }

    #[test]
    fn the_winning_class_returns_its_first_exact_page_size() {
        let first = PageSize::new(595.2, 841.6).expect("first page size should be valid");
        let same_class = PageSize::new(595.4, 841.8).expect("same-class page size should be valid");
        let document = document_with(
            &[(10, 0), (10, 1), (10, 2)],
            vec![pdf_source(10, vec![first, same_class, letter()])],
        );

        assert_eq!(document.dominant_page_size().width_pt(), 595.2);
    }

    #[test]
    fn every_permutation_of_the_non_transitive_witness_has_the_same_dominant_class() {
        let page_sizes = vec![
            PageSize::new(595.0, 800.0).expect("page size should be valid"),
            PageSize::new(595.5, 800.0).expect("page size should be valid"),
            PageSize::new(596.0, 800.0).expect("page size should be valid"),
        ];
        let permutations = [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ];
        let expected = PageSize::new(596.0, 800.0)
            .expect("expected page size should be valid")
            .size_class();

        for permutation in permutations {
            let pages = permutation.map(|page_index| (10, page_index));
            let document = document_with(&pages, vec![pdf_source(10, page_sizes.clone())]);
            assert_eq!(document.dominant_page_size().size_class(), expected);
        }
    }

    #[test]
    fn a_plan_without_pdf_pages_falls_back_to_a4_portrait() {
        let document = document_with(&[(30, 0)], vec![image_source(30)]);
        assert_eq!(
            document.dominant_page_size().size_class(),
            PageSize::A4_PORTRAIT.size_class()
        );
    }

    #[test]
    fn an_empty_plan_falls_back_to_a4_portrait() {
        assert_eq!(
            MergeDocument::default().dominant_page_size().size_class(),
            PageSize::A4_PORTRAIT.size_class()
        );
    }

    #[test]
    fn only_slots_present_in_the_plan_are_counted() {
        // The source knows about 3 pages, but only its Letter page is in the plan.
        let document = document_with(&[(10, 2)], vec![pdf_source(10, vec![A4, A4, letter()])]);
        assert_eq!(
            document.dominant_page_size().size_class(),
            letter().size_class()
        );
    }

    #[test]
    fn rotating_a_slot_does_not_change_the_dominant_page_size() {
        let plan = plan_with(&[(10, 0), (10, 1)]);
        let turned = operations::rotate(&plan, &[SlotId(0), SlotId(1)], 1);
        let document = MergeDocument::new(turned, vec![pdf_source(10, vec![A4; 2])]);

        assert_eq!(document.dominant_page_size().size_class(), A4.size_class());
    }

    #[test]
    fn an_untouched_image_keeps_its_size_when_a_pdf_page_is_turned() {
        let plan = plan_with(&[(10, 0), (30, 0)]);
        let turned = operations::rotate(&plan, &[SlotId(0)], 1);
        let document = MergeDocument::new(turned, vec![pdf_source(10, vec![A4]), image_source(30)]);

        assert_eq!(document.dominant_page_size().size_class(), A4.size_class());
    }

    proptest! {
        /// Shuffling cannot change a uniquely dominant size class.
        #[test]
        fn shuffling_the_plan_preserves_the_dominant_size_class(
            (page_sizes, shuffled_indices) in page_sizes_with_shuffle()
        ) {
            let ordered_pages: Vec<(u64, u32)> = (0..page_sizes.len())
                .map(|page_index| (10, page_index as u32))
                .collect();
            let shuffled_pages: Vec<(u64, u32)> = shuffled_indices
                .into_iter()
                .map(|page_index| (10, page_index as u32))
                .collect();
            let ordered = document_with(&ordered_pages, vec![pdf_source(10, page_sizes.clone())])
                .dominant_page_size()
                .size_class();
            let shuffled = document_with(&shuffled_pages, vec![pdf_source(10, page_sizes)])
                .dominant_page_size()
                .size_class();

            prop_assert_eq!(ordered, shuffled);
        }
    }
}
