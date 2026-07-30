use super::ids::{PageIndex, SlotId, SourceId};

/// A page from a source at a specific position in a merge plan.
#[derive(Debug, Clone, PartialEq)]
pub struct PageSlot {
    pub id: SlotId,
    pub source: SourceId,
    pub page: PageIndex,
}

/// The canonical ordered page sequence. Immutable: every operation returns a
/// new plan so that undo/redo is a plain stack of plans.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MergePlan {
    slots: Vec<PageSlot>,
}

impl MergePlan {
    /// Creates a plan with the provided ordered slots.
    pub fn new(slots: Vec<PageSlot>) -> Self {
        Self { slots }
    }

    /// Returns the plan's ordered slots.
    pub fn slots(&self) -> &[PageSlot] {
        &self.slots
    }

    /// Returns the number of slots in the plan.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Returns whether the plan has no slots.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Returns the position of a slot identifier in the plan.
    pub fn position_of(&self, id: SlotId) -> Option<usize> {
        self.slots.iter().position(|slot| slot.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(id: u64, source: u64, page: u32) -> PageSlot {
        PageSlot {
            id: SlotId(id),
            source: SourceId(source),
            page: PageIndex(page),
        }
    }

    #[test]
    fn position_of_finds_a_slot_by_id() {
        let plan = MergePlan::new(vec![slot(1, 10, 0), slot(2, 10, 1)]);
        assert_eq!(plan.position_of(SlotId(2)), Some(1));
        assert_eq!(plan.position_of(SlotId(99)), None);
    }

    #[test]
    fn the_same_page_may_appear_twice_with_distinct_slot_ids() {
        // Duplicating a cover page must be representable.
        let plan = MergePlan::new(vec![slot(1, 10, 0), slot(2, 10, 0)]);
        assert_eq!(plan.len(), 2);
        assert_ne!(plan.slots()[0].id, plan.slots()[1].id);
    }
}
