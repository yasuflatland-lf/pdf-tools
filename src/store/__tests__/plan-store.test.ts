import { beforeEach, describe, expect, it } from "vitest";
import type { SourceFileDto } from "../../bindings/SourceFileDto";
import { createPlanStore } from "../plan-store";
import { useUiStore } from "../ui-store";

function source(id: number, grouping: string): SourceFileDto {
  return {
    id,
    path: `/documents/${id}.pdf`,
    file_name: `${id}.pdf`,
    kind: "pdf",
    grouping,
    page_count: 3,
    status: { kind: "ready" },
  };
}

// `setSnapshot` reaches into the singleton UI store, so each test starts from a
// known expansion state rather than whatever the previous one left behind.
beforeEach(() => {
  useUiStore.setState({ expandedSources: new Set() });
});

describe("createPlanStore", () => {
  it("replaces its contents with the snapshot returned by Rust", () => {
    const store = createPlanStore();

    store.getState().setSnapshot({
      slots: [{ id: 1, source: 10, page: 0 }],
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

  it("prunes expansion state on every snapshot update", () => {
    useUiStore.getState().toggleExpanded(10);
    const store = createPlanStore();

    store.getState().setSnapshot({
      slots: [],
      sources: [],
      can_undo: false,
      can_redo: true,
    });

    expect(useUiStore.getState().expandedSources.size).toBe(0);
    expect(store.getState().canRedo).toBe(true);
  });

  it("re-collapses a source that is grouped again after being ungrouped", () => {
    const store = createPlanStore();
    useUiStore.getState().toggleExpanded(10);

    // Dropping an image between the pages ungroups the source ...
    store.getState().setSnapshot({
      slots: [],
      sources: [source(10, "ungrouped")],
      can_undo: true,
      can_redo: false,
    });
    expect(useUiStore.getState().expandedSources.size).toBe(0);

    // ... and deleting that image groups it again, back to a single card.
    store.getState().setSnapshot({
      slots: [],
      sources: [source(10, "grouped")],
      can_undo: true,
      can_redo: false,
    });

    expect(useUiStore.getState().expandedSources.has(10)).toBe(false);
  });
});

describe("useUiStore", () => {
  it("starts with the UI-only selections and grid view", () => {
    expect(useUiStore.getState()).toEqual({
      expandedSources: new Set(),
      selectedSlots: new Set(),
      viewMode: "grid",
      toggleExpanded: expect.any(Function),
      pruneExpanded: expect.any(Function),
    });
  });
});
