import { beforeEach, describe, expect, it, vi } from "vitest";
import type { PlanSnapshot } from "../../bindings/PlanSnapshot";
import { createPlanStore } from "../plan-store";

const { rotateSlots } = vi.hoisted(() => ({ rotateSlots: vi.fn() }));

vi.mock("../../lib/tauri-api", () => ({ rotateSlots }));

describe("createPlanStore", () => {
  beforeEach(() => {
    rotateSlots.mockReset();
  });

  it("replaces its contents with the snapshot returned by Rust", () => {
    const store = createPlanStore();

    store.getState().setSnapshot({
      slots: [{ id: 1, source: 10, page: 0, rotation: 0 }],
      sources: [],
      can_undo: false,
      can_redo: false,
    });

    expect(store.getState().slots).toHaveLength(1);
    expect(store.getState().canUndo).toBe(false);
    expect(store.getState().canRedo).toBe(false);
  });

  it("only exposes snapshot-backed plan mutations", () => {
    const store = createPlanStore();

    expect(Object.keys(store.getState())).toEqual([
      "slots",
      "sources",
      "canUndo",
      "canRedo",
      "setSnapshot",
      "rotate",
    ]);
  });

  it("rotates the selected slot ids through Rust and replaces the snapshot", async () => {
    const store = createPlanStore();
    const snapshot: PlanSnapshot = {
      slots: [{ id: 2, source: 10, page: 0, rotation: 3 }],
      sources: [],
      can_undo: true,
      can_redo: false,
    };
    rotateSlots.mockResolvedValue(snapshot);

    await store.getState().rotate([2, 4], -1);

    expect(rotateSlots).toHaveBeenCalledWith([2, 4], -1);
    expect(store.getState().slots).toBe(snapshot.slots);
    expect(store.getState().canUndo).toBe(true);
  });
});
