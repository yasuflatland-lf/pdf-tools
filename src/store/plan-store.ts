import { create } from "zustand";
import type { PageSlotDto } from "../bindings/PageSlotDto";
import type { PlanSnapshot } from "../bindings/PlanSnapshot";
import type { SourceFileDto } from "../bindings/SourceFileDto";
import { rotateSlots } from "../lib/tauri-api";

/**
 * The plan is canonical in Rust: every command returns a whole snapshot, so the
 * store only ever replaces its contents. Mutating actions delegate to Rust and
 * install the returned snapshot; they never reproduce merge rules locally.
 */
interface PlanState {
  slots: PageSlotDto[];
  sources: SourceFileDto[];
  canUndo: boolean;
  canRedo: boolean;
  setSnapshot: (snapshot: PlanSnapshot) => void;
  rotate: (slotIds: number[], delta: number) => Promise<void>;
}

export function createPlanStore() {
  return create<PlanState>((set, get) => ({
    slots: [],
    sources: [],
    canUndo: false,
    canRedo: false,
    setSnapshot: (snapshot) =>
      set({
        slots: snapshot.slots,
        sources: snapshot.sources,
        canUndo: snapshot.can_undo,
        canRedo: snapshot.can_redo,
      }),
    rotate: async (slotIds, delta) => {
      get().setSnapshot(await rotateSlots(slotIds, delta));
    },
  }));
}

export const usePlanStore = createPlanStore();
