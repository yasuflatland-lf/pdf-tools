use std::ops::Range;

use super::{
    ids::SlotId,
    plan::{MergePlan, PageSlot},
};

/// Inserts slots at `at`. Out-of-range positions are clamped to the end rather
/// than panicking: these indices originate in the UI and a stale value must not
/// crash the app.
pub fn insert_at(plan: &MergePlan, at: usize, slots: &[PageSlot]) -> MergePlan {
    let at = at.min(plan.len());
    let mut result = plan.slots().to_vec();
    result.splice(at..at, slots.iter().cloned());
    MergePlan::new(result)
}

/// Removes every slot whose id appears in `ids`. Unknown ids are ignored.
pub fn remove(plan: &MergePlan, ids: &[SlotId]) -> MergePlan {
    MergePlan::new(
        plan.slots()
            .iter()
            .filter(|slot| !ids.contains(&slot.id))
            .cloned()
            .collect(),
    )
}

/// Turns the named slots, leaving every other slot untouched. Unknown ids are
/// ignored, like every other operation over slot ids.
pub fn rotate(plan: &MergePlan, ids: &[SlotId], delta: i32) -> MergePlan {
    MergePlan::new(
        plan.slots()
            .iter()
            .map(|slot| {
                let mut slot = slot.clone();
                if ids.contains(&slot.id) {
                    slot.rotation = slot.rotation.turned(delta);
                }
                slot
            })
            .collect(),
    )
}

/// Moves the contiguous range `from` so that it begins at index `to` **in the
/// sequence that remains after the range is lifted out**. Both bounds are
/// clamped. This definition removes the usual ambiguity about whether `to`
/// refers to the pre- or post-removal indexing.
pub fn reorder(plan: &MergePlan, from: Range<usize>, to: usize) -> MergePlan {
    let start = from.start.min(plan.len());
    let end = from.end.min(plan.len());
    if start >= end {
        return plan.clone();
    }

    let mut remaining = Vec::with_capacity(plan.len());
    remaining.extend_from_slice(&plan.slots()[..start]);
    remaining.extend_from_slice(&plan.slots()[end..]);

    let to = to.min(remaining.len());
    remaining.splice(to..to, plan.slots()[start..end].iter().cloned());
    MergePlan::new(remaining)
}

#[cfg(test)]
#[allow(clippy::unnecessary_to_owned)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::domain::{
        ids::{PageIndex, SourceId},
        plan::Rotation,
    };

    fn plan_of(ids: &[u64]) -> MergePlan {
        MergePlan::new(
            ids.iter()
                .map(|&i| PageSlot {
                    id: SlotId(i),
                    source: SourceId(i / 10),
                    page: PageIndex(i as u32 % 10),
                    rotation: Rotation::default(),
                })
                .collect(),
        )
    }
    fn ids_of(plan: &MergePlan) -> Vec<u64> {
        plan.slots().iter().map(|s| s.id.0).collect()
    }

    #[test]
    fn insert_at_places_slots_at_the_given_index() {
        let plan = plan_of(&[1, 2, 3]);
        let new = insert_at(&plan, 1, &plan_of(&[9]).slots().to_vec());
        assert_eq!(ids_of(&new), vec![1, 9, 2, 3]);
    }

    #[test]
    fn insert_at_clamps_an_out_of_range_index_to_the_end() {
        let plan = plan_of(&[1, 2]);
        let new = insert_at(&plan, 99, &plan_of(&[9]).slots().to_vec());
        assert_eq!(ids_of(&new), vec![1, 2, 9]);
    }

    #[test]
    fn remove_drops_the_named_slots_and_ignores_unknown_ids() {
        let new = remove(&plan_of(&[1, 2, 3]), &[SlotId(2), SlotId(99)]);
        assert_eq!(ids_of(&new), vec![1, 3]);
    }

    #[test]
    fn rotate_returns_a_new_plan_and_leaves_the_original_untouched() {
        let plan = plan_of(&[1, 2]);
        let new = rotate(&plan, &[SlotId(1)], 1);

        assert_eq!(plan.slots()[0].rotation, Rotation::default());
        assert_eq!(new.slots()[0].rotation.quarter_turns(), 1);
        assert_eq!(new.slots()[1].rotation, Rotation::default());
    }

    #[test]
    fn rotate_accepts_a_negative_delta() {
        let plan = plan_of(&[1]);
        let negative = rotate(&plan, &[SlotId(1)], -1);

        assert_eq!(negative.slots()[0].rotation.quarter_turns(), 3);
        assert_eq!(negative, rotate(&plan, &[SlotId(1)], 3));
    }

    #[test]
    fn four_turns_restore_the_original_rotation() {
        let original = plan_of(&[1]);
        let once = rotate(&original, &[SlotId(1)], 1);
        let twice = rotate(&once, &[SlotId(1)], 1);
        let three_times = rotate(&twice, &[SlotId(1)], 1);
        let four_times = rotate(&three_times, &[SlotId(1)], 1);

        assert_eq!(four_times, original);
    }

    #[test]
    fn rotate_ignores_empty_and_unknown_id_lists() {
        let plan = plan_of(&[1, 2]);
        assert_eq!(rotate(&plan, &[], 1), plan);
        assert_eq!(rotate(&plan, &[SlotId(99)], 1), plan);
    }

    #[test]
    fn reorder_moves_a_range_forward() {
        // Lift [1,2] out -> [3,4,5]; insert at index 2 -> [3,4,1,2,5]
        let new = reorder(&plan_of(&[1, 2, 3, 4, 5]), 0..2, 2);
        assert_eq!(ids_of(&new), vec![3, 4, 1, 2, 5]);
    }

    #[test]
    fn reorder_moves_a_range_backward() {
        // Lift [4,5] out -> [1,2,3]; insert at index 1 -> [1,4,5,2,3]
        let new = reorder(&plan_of(&[1, 2, 3, 4, 5]), 3..5, 1);
        assert_eq!(ids_of(&new), vec![1, 4, 5, 2, 3]);
    }

    #[test]
    fn reorder_to_the_same_position_is_a_no_op() {
        let plan = plan_of(&[1, 2, 3]);
        assert_eq!(reorder(&plan, 1..2, 1), plan);
    }

    #[test]
    fn reorder_clamps_out_of_range_bounds() {
        let plan = plan_of(&[1, 2, 3]);
        let new = reorder(&plan, 0..1, 99);
        assert_eq!(ids_of(&new), vec![2, 3, 1]);
    }

    #[test]
    fn reorder_leaves_the_plan_alone_for_a_degenerate_range() {
        // A stale UI selection may yield an empty, reversed, or wholly
        // out-of-range span. None of these may panic.
        let plan = plan_of(&[1, 2, 3]);
        // Built field-by-field because a reversed literal range is a lint error.
        let reversed = Range { start: 2, end: 1 };
        assert_eq!(reorder(&plan, 1..1, 0), plan);
        assert_eq!(reorder(&plan, reversed, 0), plan);
        assert_eq!(reorder(&plan, 90..99, 0), plan);
    }

    #[test]
    fn the_operations_tolerate_an_empty_plan() {
        let empty = MergePlan::default();
        assert_eq!(
            ids_of(&insert_at(&empty, 42, plan_of(&[9]).slots())),
            vec![9]
        );
        assert_eq!(remove(&empty, &[SlotId(1)]), empty);
        assert_eq!(reorder(&empty, 0..3, 7), empty);
        assert_eq!(rotate(&empty, &[SlotId(1)], 1), empty);
    }

    proptest! {
        /// Reordering never invents, drops, or duplicates a slot.
        #[test]
        fn reorder_preserves_the_multiset_of_slots(
            len in 1usize..30, start in 0usize..30, take in 1usize..10, to in 0usize..30
        ) {
            let ids: Vec<u64> = (0..len as u64).collect();
            let plan = plan_of(&ids);
            let start = start.min(len - 1);
            let end = (start + take).min(len);
            let new = reorder(&plan, start..end, to);
            let mut before = ids_of(&plan); before.sort_unstable();
            let mut after = ids_of(&new);  after.sort_unstable();
            prop_assert_eq!(before, after);
        }

        /// Removal shrinks the plan by exactly the number of ids that were present.
        #[test]
        fn remove_shrinks_by_the_number_of_present_ids(len in 0usize..30, victims in prop::collection::vec(0u64..40, 0..10)) {
            let ids: Vec<u64> = (0..len as u64).collect();
            let plan = plan_of(&ids);
            let present: std::collections::BTreeSet<u64> =
                victims.iter().copied().filter(|v| (*v as usize) < len).collect();
            let new = remove(&plan, &victims.iter().map(|&v| SlotId(v)).collect::<Vec<_>>());
            prop_assert_eq!(new.len(), len - present.len());
        }

        /// Insertion grows the plan by exactly the number of inserted slots.
        #[test]
        fn insert_grows_by_the_number_of_inserted_slots(len in 0usize..30, at in 0usize..40, count in 0usize..5) {
            let plan = plan_of(&(0..len as u64).collect::<Vec<_>>());
            let extra = plan_of(&(100..100 + count as u64).collect::<Vec<_>>());
            let new = insert_at(&plan, at, extra.slots());
            prop_assert_eq!(new.len(), len + count);
        }
    }
}
