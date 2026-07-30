import type { PageSlotDto } from "../bindings/PageSlotDto";
import type { PlanSnapshot } from "../bindings/PlanSnapshot";
import type { SourceFileDto } from "../bindings/SourceFileDto";
import { createStore } from "./create-store";

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
  return createStore<PlanState>((setState) => ({
    slots: [],
    sources: [],
    setSnapshot: (snapshot) =>
      setState((state) => ({
        slots: snapshot.slots,
        sources: snapshot.sources,
        setSnapshot: state.setSnapshot,
      })),
  }));
}

export const usePlanStore = createPlanStore();
