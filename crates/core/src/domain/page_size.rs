use std::collections::HashMap;

use super::geometry::{PageSize, SizeClass};
use super::plan::MergePlan;
use super::source::{SourceFile, SourceKind};

/// The page size that images are fitted to: the most frequent size among the
/// plan's PDF-backed slots. Ties go to whichever size appears first in the
/// plan. Falls back to A4 portrait when the plan contains no PDF pages.
/// Sizes are classified on a one-point lattice.
pub fn dominant_page_size(plan: &MergePlan, sources: &[SourceFile]) -> PageSize {
    let buckets = plan
        .slots()
        .iter()
        .enumerate()
        .filter_map(|(plan_index, slot)| {
            let source = sources.iter().find(|source| source.id == slot.source)?;
            if source.kind != SourceKind::Pdf {
                return None;
            }
            source
                .page_sizes
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use proptest::prelude::*;

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

    fn page_sizes_with_shuffle() -> impl Strategy<Value = (Vec<PageSize>, Vec<usize>)> {
        let page_size = || {
            (1.0f32..2000.0, 1.0f32..2000.0).prop_map(|(width_pt, height_pt)| PageSize {
                width_pt,
                height_pt,
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
        assert_eq!(
            dominant_page_size(&plan, &sources).size_class(),
            A4.size_class()
        );
    }

    #[test]
    fn a_tie_is_broken_by_first_appearance_in_the_plan() {
        let plan = plan_with(&[(10, 0), (10, 1)]);
        let sources = vec![pdf_source(10, vec![LETTER, A4])];
        assert_eq!(
            dominant_page_size(&plan, &sources).size_class(),
            LETTER.size_class()
        );
    }

    #[test]
    fn sizes_in_the_same_lattice_cell_are_counted_together() {
        let almost_a4 = PageSize {
            width_pt: 595.4,
            height_pt: 841.6,
        };
        let plan = plan_with(&[(10, 0), (10, 1), (10, 2)]);
        let sources = vec![pdf_source(10, vec![A4, almost_a4, LETTER])];
        // A4 and almost-A4 form one bucket of 2, beating Letter's 1.
        assert_eq!(
            dominant_page_size(&plan, &sources).size_class(),
            A4.size_class()
        );
    }

    #[test]
    fn a_lattice_cell_of_near_equal_sizes_outvotes_an_earlier_lone_size() {
        let almost_a4 = PageSize {
            width_pt: 595.4,
            height_pt: 841.6,
        };
        let plan = plan_with(&[(10, 0), (10, 1), (10, 2)]);
        let sources = vec![pdf_source(10, vec![LETTER, A4, almost_a4])];
        // Letter appears first but stands alone; A4 and almost-A4 share a bucket.
        assert_eq!(
            dominant_page_size(&plan, &sources).size_class(),
            A4.size_class()
        );
    }

    #[test]
    fn the_winning_class_returns_its_first_exact_page_size() {
        let first = PageSize {
            width_pt: 595.2,
            height_pt: 841.6,
        };
        let same_class = PageSize {
            width_pt: 595.4,
            height_pt: 841.8,
        };
        let plan = plan_with(&[(10, 0), (10, 1), (10, 2)]);
        let sources = vec![pdf_source(10, vec![first, same_class, LETTER])];

        assert_eq!(dominant_page_size(&plan, &sources), first);
    }

    #[test]
    fn every_permutation_of_the_non_transitive_witness_has_the_same_dominant_class() {
        let page_sizes = vec![
            PageSize {
                width_pt: 595.0,
                height_pt: 800.0,
            },
            PageSize {
                width_pt: 595.5,
                height_pt: 800.0,
            },
            PageSize {
                width_pt: 596.0,
                height_pt: 800.0,
            },
        ];
        let sources = vec![pdf_source(10, page_sizes)];
        let permutations = [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ];
        let expected = PageSize {
            width_pt: 596.0,
            height_pt: 800.0,
        }
        .size_class();

        for permutation in permutations {
            let pages = permutation.map(|page_index| (10, page_index));
            let plan = plan_with(&pages);
            assert_eq!(dominant_page_size(&plan, &sources).size_class(), expected);
        }
    }

    #[test]
    fn a_plan_without_pdf_pages_falls_back_to_a4_portrait() {
        let plan = plan_with(&[(30, 0)]);
        let sources = vec![image_source(30)];
        assert_eq!(
            dominant_page_size(&plan, &sources).size_class(),
            PageSize::A4_PORTRAIT.size_class()
        );
    }

    #[test]
    fn an_empty_plan_falls_back_to_a4_portrait() {
        assert_eq!(
            dominant_page_size(&MergePlan::default(), &[]).size_class(),
            PageSize::A4_PORTRAIT.size_class()
        );
    }

    #[test]
    fn only_slots_present_in_the_plan_are_counted() {
        // The source knows about 3 pages, but only its Letter page is in the plan.
        let plan = plan_with(&[(10, 2)]);
        let sources = vec![pdf_source(10, vec![A4, A4, LETTER])];
        assert_eq!(
            dominant_page_size(&plan, &sources).size_class(),
            LETTER.size_class()
        );
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
            let sources = vec![pdf_source(10, page_sizes)];
            let ordered = dominant_page_size(&plan_with(&ordered_pages), &sources).size_class();
            let shuffled = dominant_page_size(&plan_with(&shuffled_pages), &sources).size_class();

            prop_assert_eq!(ordered, shuffled);
        }
    }
}
