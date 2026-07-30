use super::geometry::PageSize;
use super::plan::MergePlan;
use super::source::{SourceFile, SourceKind};

/// The page size that images are fitted to: the most frequent size among the
/// plan's PDF-backed slots. Ties go to whichever size appears first in the
/// plan. Falls back to A4 portrait when the plan contains no PDF pages.
/// Sizes within 1 pt of each other count as the same size.
pub fn dominant_page_size(plan: &MergePlan, sources: &[SourceFile]) -> PageSize {
    let mut buckets: Vec<(PageSize, usize)> = Vec::new();

    for slot in plan.slots() {
        let Some(source) = sources.iter().find(|source| source.id == slot.source) else {
            continue;
        };
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::domain::ids::{PageIndex, SlotId, SourceId};
    use crate::domain::plan::PageSlot;
    use crate::domain::source::{Grouping, SourceStatus};

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
            grouping: Grouping::Grouped,
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
            grouping: Grouping::Ungrouped,
            page_count: 1,
            page_sizes: vec![LETTER],
            status: SourceStatus::Ready,
        }
    }

    #[test]
    fn the_most_frequent_pdf_page_size_wins() {
        // Two A4 pages, one Letter page.
        let plan = plan_with(&[(10, 0), (10, 1), (10, 2)]);
        let sources = vec![pdf_source(10, vec![A4, A4, LETTER])];
        assert!(dominant_page_size(&plan, &sources).approx_eq(&A4));
    }

    #[test]
    fn a_tie_is_broken_by_first_appearance_in_the_plan() {
        let plan = plan_with(&[(10, 0), (10, 1)]);
        let sources = vec![pdf_source(10, vec![LETTER, A4])];
        assert!(dominant_page_size(&plan, &sources).approx_eq(&LETTER));
    }

    #[test]
    fn sizes_within_one_point_are_counted_together() {
        let almost_a4 = PageSize {
            width_pt: 595.9,
            height_pt: 841.2,
        };
        let plan = plan_with(&[(10, 0), (10, 1), (10, 2)]);
        let sources = vec![pdf_source(10, vec![A4, almost_a4, LETTER])];
        // A4 and almost-A4 form one bucket of 2, beating Letter's 1.
        assert!(dominant_page_size(&plan, &sources).approx_eq(&A4));
    }

    #[test]
    fn a_bucket_of_near_equal_sizes_outvotes_an_earlier_lone_size() {
        let almost_a4 = PageSize {
            width_pt: 595.9,
            height_pt: 841.2,
        };
        let plan = plan_with(&[(10, 0), (10, 1), (10, 2)]);
        let sources = vec![pdf_source(10, vec![LETTER, A4, almost_a4])];
        // Letter appears first but stands alone; A4 and almost-A4 share a bucket.
        assert!(dominant_page_size(&plan, &sources).approx_eq(&A4));
    }

    #[test]
    fn a_plan_without_pdf_pages_falls_back_to_a4_portrait() {
        let plan = plan_with(&[(30, 0)]);
        let sources = vec![image_source(30)];
        assert!(dominant_page_size(&plan, &sources).approx_eq(&PageSize::A4_PORTRAIT));
    }

    #[test]
    fn an_empty_plan_falls_back_to_a4_portrait() {
        assert!(dominant_page_size(&MergePlan::default(), &[]).approx_eq(&PageSize::A4_PORTRAIT));
    }

    #[test]
    fn only_slots_present_in_the_plan_are_counted() {
        // The source knows about 3 pages, but only its Letter page is in the plan.
        let plan = plan_with(&[(10, 2)]);
        let sources = vec![pdf_source(10, vec![A4, A4, LETTER])];
        assert!(dominant_page_size(&plan, &sources).approx_eq(&LETTER));
    }
}
