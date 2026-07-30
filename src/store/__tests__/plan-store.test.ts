import { describe, expect, it } from "vitest";
import { createPlanStore } from "../plan-store";
import { useUiStore } from "../ui-store";

describe("createPlanStore", () => {
  it("replaces its contents with the snapshot returned by Rust", () => {
    const store = createPlanStore();

    store.getState().setSnapshot({
      slots: [{ id: 1, source: 10, page: 0 }],
      sources: [],
    });

    expect(store.getState().slots).toHaveLength(1);
  });

  it("never mutates the plan locally", () => {
    const store = createPlanStore();

    expect(Object.keys(store.getState())).toEqual(["slots", "sources", "setSnapshot"]);
  });
});

describe("useUiStore", () => {
  it("starts with the UI-only selections and grid view", () => {
    expect(useUiStore.getState()).toEqual({
      expandedSources: new Set(),
      selectedSlots: new Set(),
      viewMode: "grid",
    });
  });
});
