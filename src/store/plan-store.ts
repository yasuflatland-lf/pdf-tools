import { create } from "zustand";
import type { PageSlotDto } from "../bindings/PageSlotDto";
import type { PlanSnapshot } from "../bindings/PlanSnapshot";
import type { SourceFileDto } from "../bindings/SourceFileDto";
import { useUiStore } from "./ui-store";

/**
 * The plan is canonical in Rust: every command returns a whole snapshot, so the
 * store only ever replaces its contents. It deliberately exposes no reorder,
 * insert or remove helper -- a local implementation of those would be a second,
 * divergent copy of the merge rules.
 */
interface PlanState {
  slots: PageSlotDto[];
  sources: SourceFileDto[];
  canUndo: boolean;
  canRedo: boolean;
  setSnapshot: (snapshot: PlanSnapshot) => void;
}

export function createPlanStore() {
  return create<PlanState>((set) => ({
    slots: [],
    sources: [],
    canUndo: false,
    canRedo: false,
    setSnapshot: (snapshot) => {
      // Expansion only means something for a source that is still present and
      // still grouped. Pruning against that list drops the entry of a source an
      // undo removed, and re-collapses one that was ungrouped and then grouped
      // again -- otherwise a stale flag would keep it exploded into page cards.
      useUiStore
        .getState()
        .pruneExpanded(
          snapshot.sources
            .filter((source) => source.grouping === "grouped")
            .map((source) => source.id),
        );
      set({
        slots: snapshot.slots,
        sources: snapshot.sources,
        canUndo: snapshot.can_undo,
        canRedo: snapshot.can_redo,
      });
    },
  }));
}

export const usePlanStore = createPlanStore();
