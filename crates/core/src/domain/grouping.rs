use super::ids::SourceId;
use super::plan::MergePlan;
use super::source::{Grouping, SourceFile};

/// A contiguous range of slots represented as one group.
#[derive(Debug, Clone, PartialEq)]
pub struct SlotGroup {
    pub source: SourceId,
    pub start: usize,
    pub len: usize,
}

/// Collapses runs of contiguous slots that share a source. Sources marked
/// ungrouped yield one single-slot group per slot.
pub fn group_contiguous(plan: &MergePlan, sources: &[SourceFile]) -> Vec<SlotGroup> {
    let mut groups: Vec<SlotGroup> = Vec::new();

    for (index, slot) in plan.slots().iter().enumerate() {
        let is_grouped = sources
            .iter()
            .find(|source| source.id == slot.source)
            .is_some_and(|source| source.grouping == Grouping::Grouped);

        if is_grouped {
            if let Some(group) = groups.last_mut() {
                if group.source == slot.source {
                    group.len += 1;
                    continue;
                }
            }
        }

        groups.push(SlotGroup {
            source: slot.source,
            start: index,
            len: 1,
        });
    }

    groups
}

/// Returns whether every slot of `source` sits in one contiguous run whose page
/// numbers strictly ascend. Missing pages in that sequence are allowed.
pub fn can_regroup(plan: &MergePlan, source: SourceId) -> bool {
    let mut previous_page = None;
    let mut run_ended = false;

    for slot in plan.slots() {
        if slot.source == source {
            if run_ended || previous_page.is_some_and(|previous| slot.page <= previous) {
                return false;
            }
            previous_page = Some(slot.page);
        } else if previous_page.is_some() {
            run_ended = true;
        }
    }

    true
}

/// Re-evaluates every source's grouping after a plan change. A source is
/// grouped exactly when [`can_regroup`] returns true.
pub fn reconcile_grouping(plan: &MergePlan, sources: &mut [SourceFile]) {
    for source in sources {
        source.grouping = if can_regroup(plan, source.id) {
            Grouping::Grouped
        } else {
            Grouping::Ungrouped
        };
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use proptest::prelude::*;

    use super::*;
    use crate::domain::ids::{PageIndex, SlotId, SourceId};
    use crate::domain::plan::{MergePlan, PageSlot};
    use crate::domain::source::{Grouping, SourceFile, SourceKind, SourceStatus};

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

    fn grouped(id: u64) -> SourceFile {
        SourceFile {
            id: SourceId(id),
            path: PathBuf::new(),
            kind: SourceKind::Pdf,
            grouping: Grouping::Grouped,
            page_count: 0,
            page_sizes: Vec::new(),
            status: SourceStatus::Ready,
        }
    }

    fn ungrouped(id: u64) -> SourceFile {
        SourceFile {
            grouping: Grouping::Ungrouped,
            ..grouped(id)
        }
    }

    #[test]
    fn contiguous_slots_from_one_grouped_source_form_a_single_group() {
        let plan = plan_with(&[(10, 0), (10, 1), (20, 0)]);
        let sources = vec![grouped(10), grouped(20)];
        let groups = group_contiguous(&plan, &sources);
        assert_eq!(groups.len(), 2);
        assert_eq!(
            groups[0],
            SlotGroup {
                source: SourceId(10),
                start: 0,
                len: 2,
            }
        );
    }

    #[test]
    fn an_ungrouped_source_yields_one_group_per_slot() {
        let plan = plan_with(&[(10, 0), (10, 1)]);
        let groups = group_contiguous(&plan, &[ungrouped(10)]);
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn inserting_inside_a_group_releases_it() {
        // [10:0, 10:1, 10:2] -> insert 20:0 at index 1
        let plan = plan_with(&[(10, 0), (20, 0), (10, 1), (10, 2)]);
        let mut sources = vec![grouped(10), grouped(20)];
        reconcile_grouping(&plan, &mut sources);
        assert_eq!(sources[0].grouping, Grouping::Ungrouped);
        assert_eq!(sources[1].grouping, Grouping::Grouped);
    }

    #[test]
    fn inserting_at_a_group_boundary_does_not_release_anything() {
        // [10:0, 10:1] + [20:0] appended -- neither group is split
        let plan = plan_with(&[(10, 0), (10, 1), (20, 0)]);
        let mut sources = vec![grouped(10), grouped(20)];
        reconcile_grouping(&plan, &mut sources);
        assert_eq!(sources[0].grouping, Grouping::Grouped);
    }

    #[test]
    fn removing_the_inserted_slot_regroups_the_source_automatically() {
        let plan = plan_with(&[(10, 0), (10, 1), (10, 2)]);
        let mut sources = vec![SourceFile {
            grouping: Grouping::Ungrouped,
            ..grouped(10)
        }];
        reconcile_grouping(&plan, &mut sources);
        assert_eq!(sources[0].grouping, Grouping::Grouped);
    }

    #[test]
    fn a_gap_left_by_deletion_still_allows_regrouping() {
        // Page 1 was deleted: pages 0 and 2 remain, still ascending.
        let plan = plan_with(&[(10, 0), (10, 2)]);
        assert!(can_regroup(&plan, SourceId(10)));
    }

    #[test]
    fn reordered_pages_do_not_allow_regrouping() {
        let plan = plan_with(&[(10, 0), (10, 2), (10, 1)]);
        assert!(!can_regroup(&plan, SourceId(10)));
    }

    #[test]
    fn duplicated_pages_do_not_allow_regrouping() {
        // A duplicated cover page makes the run non-strictly-ascending, so the
        // card's page count would misrepresent the contents.
        let plan = plan_with(&[(10, 0), (10, 0), (10, 1)]);
        assert!(!can_regroup(&plan, SourceId(10)));
    }

    #[test]
    fn a_single_page_image_source_is_always_groupable() {
        let plan = plan_with(&[(30, 0)]);
        assert!(can_regroup(&plan, SourceId(30)));
    }

    #[test]
    fn an_absent_source_is_vacuously_groupable() {
        let plan = plan_with(&[(10, 0)]);
        assert!(can_regroup(&plan, SourceId(20)));
    }

    #[test]
    fn every_source_is_groupable_in_an_empty_plan() {
        assert!(can_regroup(&MergePlan::default(), SourceId(10)));
    }

    #[test]
    fn an_empty_plan_has_no_groups() {
        assert!(group_contiguous(&MergePlan::default(), &[grouped(10)]).is_empty());
    }

    #[test]
    fn unknown_sources_yield_one_group_per_slot() {
        let plan = plan_with(&[(10, 0), (10, 1), (20, 0)]);
        let groups = group_contiguous(&plan, &[grouped(20)]);
        assert_eq!(
            groups,
            vec![
                SlotGroup {
                    source: SourceId(10),
                    start: 0,
                    len: 1,
                },
                SlotGroup {
                    source: SourceId(10),
                    start: 1,
                    len: 1,
                },
                SlotGroup {
                    source: SourceId(20),
                    start: 2,
                    len: 1,
                },
            ]
        );
    }

    #[test]
    fn grouping_contiguous_slots_does_not_require_ascending_pages() {
        let plan = plan_with(&[(10, 2), (10, 0), (10, 1)]);
        let groups = group_contiguous(&plan, &[grouped(10)]);
        assert_eq!(
            groups,
            vec![SlotGroup {
                source: SourceId(10),
                start: 0,
                len: 3,
            }]
        );
    }

    proptest! {
        /// `group_contiguous` covers every slot exactly once, in order.
        #[test]
        fn groups_partition_the_plan(
            entries in prop::collection::vec((0u64..3, 0u32..5), 0..20)
        ) {
            let plan = plan_with(&entries);
            let sources: Vec<SourceFile> = (0..3).map(grouped).collect();
            let groups = group_contiguous(&plan, &sources);
            let covered: usize = groups.iter().map(|group| group.len).sum();
            prop_assert_eq!(covered, plan.len());

            // Groups are consecutive and non-overlapping.
            let mut cursor = 0;
            for group in &groups {
                prop_assert_eq!(group.start, cursor);
                cursor += group.len;
            }
        }

        /// `reconcile_grouping` is idempotent: running it twice changes nothing.
        #[test]
        fn reconcile_is_idempotent(
            entries in prop::collection::vec((0u64..3, 0u32..5), 0..20)
        ) {
            let plan = plan_with(&entries);
            let mut a: Vec<SourceFile> = (0..3).map(grouped).collect();
            reconcile_grouping(&plan, &mut a);
            let mut b = a.clone();
            reconcile_grouping(&plan, &mut b);
            prop_assert_eq!(a, b);
        }
    }
}
