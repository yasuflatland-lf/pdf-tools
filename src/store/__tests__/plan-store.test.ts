import { describe, expect, it } from "vitest";
import { createPlanStore } from "../plan-store";

describe("createPlanStore", () => {
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

  it("never mutates the plan locally", () => {
    const store = createPlanStore();

    expect(Object.keys(store.getState())).toEqual([
      "slots",
      "sources",
      "canUndo",
      "canRedo",
      "setSnapshot",
    ]);
  });
});
