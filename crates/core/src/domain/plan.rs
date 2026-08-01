use super::ids::{PageIndex, SlotId, SourceId};

/// A clockwise quarter-turn count. Four turns return to the original, so this
/// is a position and not an accumulation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Rotation(u8);

impl Rotation {
    /// Creates a rotation by normalizing a raw quarter-turn count.
    pub fn from_quarter_turns(quarter_turns: i32) -> Self {
        Self(quarter_turns.rem_euclid(4) as u8)
    }

    /// Returns this position after the given number of quarter turns.
    pub fn turned(self, delta: i8) -> Self {
        Self::from_quarter_turns(i32::from(self.0) + i32::from(delta))
    }

    /// Returns the clockwise quarter-turn count in `0..4`.
    pub fn quarter_turns(self) -> u8 {
        self.0
    }
}

/// A page from a source at a specific position in a merge plan.
#[derive(Debug, Clone, PartialEq)]
pub struct PageSlot {
    pub id: SlotId,
    pub source: SourceId,
    pub page: PageIndex,
    pub rotation: Rotation,
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
            rotation: Rotation::default(),
        }
    }

    #[test]
    fn rotation_wraps_for_negative_and_large_deltas() {
        let rotation = Rotation::default();
        assert_eq!(rotation.turned(-1).quarter_turns(), 3);
        assert_eq!(rotation.turned(6).quarter_turns(), 2);
    }

    #[test]
    fn raw_quarter_turn_counts_are_normalized() {
        assert_eq!(Rotation::from_quarter_turns(-5).quarter_turns(), 3);
        assert_eq!(Rotation::from_quarter_turns(10).quarter_turns(), 2);
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
