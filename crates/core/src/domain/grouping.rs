use super::ids::SourceId;
use super::plan::MergePlan;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ids::{PageIndex, SlotId, SourceId};
    use crate::domain::plan::{MergePlan, PageSlot};

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

    #[test]
    fn inserting_inside_a_group_releases_it() {
        // [10:0, 10:1, 10:2] -> insert 20:0 at index 1
        let plan = plan_with(&[(10, 0), (20, 0), (10, 1), (10, 2)]);
        assert!(!can_regroup(&plan, SourceId(10)));
        assert!(can_regroup(&plan, SourceId(20)));
    }

    #[test]
    fn inserting_at_a_group_boundary_does_not_release_anything() {
        // [10:0, 10:1] + [20:0] appended -- neither group is split
        let plan = plan_with(&[(10, 0), (10, 1), (20, 0)]);
        assert!(can_regroup(&plan, SourceId(10)));
        assert!(can_regroup(&plan, SourceId(20)));
    }

    #[test]
    fn removing_the_inserted_slot_regroups_the_source_automatically() {
        let plan = plan_with(&[(10, 0), (10, 1), (10, 2)]);
        assert!(can_regroup(&plan, SourceId(10)));
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
}
