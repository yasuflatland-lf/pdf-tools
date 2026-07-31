use super::geometry::PageSize;
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
            .filter(|slot| sources.iter().any(|source| source.id == slot.source))
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
            .find(|source| source.id == slot.source)
            .expect("MergeDocument construction guarantees every slot has a source")
    }

    /// The most frequent size among the document's PDF-backed slots. Ties go
    /// to the size that appears first. Sizes within 1 pt count as equal.
    pub fn dominant_page_size(&self) -> PageSize {
        let mut buckets: Vec<(PageSize, usize)> = Vec::new();

        for slot in self.plan.slots() {
            let source = self.source_of(slot);
            if source.kind != SourceKind::Pdf {
                continue;
            }
            let Some(&page_size) = source.page_sizes.get(slot.page.0 as usize) else {
                continue;
            };

            if let Some((_, count)) = buckets
                .iter_mut()
                .find(|(representative, _)| representative.approx_eq(&page_size))
            {
                *count += 1;
            } else {
                buckets.push((page_size, 1));
            }
        }

        buckets
            .into_iter()
            .fold(None::<(PageSize, usize)>, |winner, bucket| match winner {
                Some(current) if current.1 >= bucket.1 => Some(current),
                _ => Some(bucket),
            })
            .map_or(PageSize::A4_PORTRAIT, |(representative, _)| representative)
    }

    pub fn is_grouped(&self, source: SourceId) -> bool {
        can_regroup(&self.plan, source)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::domain::ids::{PageIndex, SlotId, SourceId};
    use crate::domain::source::SourceStatus;

    const A4: PageSize = PageSize::A4_PORTRAIT;
    const LETTER: PageSize = PageSize {
        width_pt: 612.0,
        height_pt: 792.0,
    };

    fn plan_with(pages: &[(u64, u32)]) -> MergePlan {
        MergePlan::new(
            pages
                .iter()
                .enumerate()
                .map(|(slot_id, &(source_id, page_index))| PageSlot {
                    id: SlotId(slot_id as u64),
                    source: SourceId(source_id),
                    page: PageIndex(page_index),
                })
                .collect(),
        )
    }

    fn pdf_source(id: u64, page_sizes: Vec<PageSize>) -> SourceFile {
        SourceFile {
            id: SourceId(id),
            path: PathBuf::new(),
            kind: SourceKind::Pdf,
            page_count: page_sizes.len() as u32,
            page_sizes,
            status: SourceStatus::Ready,
        }
    }

    /// The single page size is deliberately not A4 so that a test expecting the
    /// A4 fallback fails if image sources were ever counted.
    fn image_source(id: u64) -> SourceFile {
        SourceFile {
            id: SourceId(id),
            path: PathBuf::new(),
            kind: SourceKind::Image,
            page_count: 1,
            page_sizes: vec![LETTER],
            status: SourceStatus::Ready,
        }
    }

    #[test]
    fn construction_drops_slots_whose_source_is_absent() {
        let orphan = PageSlot {
            id: SlotId(1),
            source: SourceId(99),
            page: PageIndex(0),
        };

        let document = MergeDocument::new(MergePlan::new(vec![orphan]), Vec::new());

        assert!(document.plan().is_empty());
    }

    #[test]
    fn source_of_is_total_for_a_slot_in_the_document() {
        let document = MergeDocument::new(plan_with(&[(10, 0)]), vec![pdf_source(10, vec![A4])]);

        assert_eq!(
            document.source_of(&document.plan().slots()[0]).id,
            SourceId(10)
        );
    }

    #[test]
    fn the_most_frequent_pdf_page_size_wins() {
        let document = MergeDocument::new(
            plan_with(&[(10, 0), (10, 1), (10, 2)]),
            vec![pdf_source(10, vec![A4, A4, LETTER])],
        );
        assert!(document.dominant_page_size().approx_eq(&A4));
    }

    #[test]
    fn a_tie_is_broken_by_first_appearance_in_the_plan() {
        let document = MergeDocument::new(
            plan_with(&[(10, 0), (10, 1)]),
            vec![pdf_source(10, vec![LETTER, A4])],
        );
        assert!(document.dominant_page_size().approx_eq(&LETTER));
    }

    #[test]
    fn sizes_within_one_point_are_counted_together() {
        let almost_a4 = PageSize {
            width_pt: 595.9,
            height_pt: 841.2,
        };
        let document = MergeDocument::new(
            plan_with(&[(10, 0), (10, 1), (10, 2)]),
            vec![pdf_source(10, vec![A4, almost_a4, LETTER])],
        );
        assert!(document.dominant_page_size().approx_eq(&A4));
    }

    #[test]
    fn a_bucket_of_near_equal_sizes_outvotes_an_earlier_lone_size() {
        let almost_a4 = PageSize {
            width_pt: 595.9,
            height_pt: 841.2,
        };
        let document = MergeDocument::new(
            plan_with(&[(10, 0), (10, 1), (10, 2)]),
            vec![pdf_source(10, vec![LETTER, A4, almost_a4])],
        );
        assert!(document.dominant_page_size().approx_eq(&A4));
    }

    #[test]
    fn a_plan_without_pdf_pages_falls_back_to_a4_portrait() {
        let document = MergeDocument::new(plan_with(&[(30, 0)]), vec![image_source(30)]);
        assert!(document
            .dominant_page_size()
            .approx_eq(&PageSize::A4_PORTRAIT));
    }

    #[test]
    fn an_empty_plan_falls_back_to_a4_portrait() {
        assert!(MergeDocument::default()
            .dominant_page_size()
            .approx_eq(&PageSize::A4_PORTRAIT));
    }

    #[test]
    fn only_slots_present_in_the_plan_are_counted() {
        let document = MergeDocument::new(
            plan_with(&[(10, 2)]),
            vec![pdf_source(10, vec![A4, A4, LETTER])],
        );
        assert!(document.dominant_page_size().approx_eq(&LETTER));
    }
}
