/// Identifies a page slot in a merge plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SlotId(pub u64);

/// Identifies a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourceId(pub u64);

/// A zero-based page index within a source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PageIndex(pub u32);

/// Hands out monotonically increasing ids. Slots and sources are numbered
/// independently so that neither sequence constrains the other.
#[derive(Debug, Default)]
pub struct IdSequence {
    next_slot: u64,
    next_source: u64,
}

impl IdSequence {
    /// Returns the next slot identifier.
    pub fn next_slot(&mut self) -> SlotId {
        let id = SlotId(self.next_slot);
        self.next_slot += 1;
        id
    }

    /// Returns the next source identifier.
    pub fn next_source(&mut self) -> SourceId {
        let id = SourceId(self.next_source);
        self.next_source += 1;
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_and_source_ids_are_numbered_independently() {
        let mut ids = IdSequence::default();
        assert_eq!(ids.next_slot(), SlotId(0));
        assert_eq!(ids.next_source(), SourceId(0));
        assert_eq!(ids.next_slot(), SlotId(1));
    }
}
