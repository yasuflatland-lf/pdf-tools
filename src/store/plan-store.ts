import { create } from "zustand";
import type { PageSlotDto } from "../bindings/PageSlotDto";
import type { PlanSnapshot } from "../bindings/PlanSnapshot";
import type { SourceFileDto } from "../bindings/SourceFileDto";

/**
 * The plan is canonical in Rust: every command returns a whole snapshot, so the
 * store only ever replaces its contents. It deliberately exposes no reorder,
 * insert or remove helper -- a local implementation of those would be a second,
 * divergent copy of the merge rules.
 */
interface PlanState {
  slots: PageSlotDto[];
  sources: SourceFileDto[];
  setSnapshot: (snapshot: PlanSnapshot) => void;
}

export function createPlanStore() {
  return create<PlanState>((set) => ({
    slots: [],
    sources: [],
    setSnapshot: (snapshot) => set({ slots: snapshot.slots, sources: snapshot.sources }),
  }));
}

export const usePlanStore = createPlanStore();
