import { describe, expect, it } from "vitest";
import { createUiStore } from "../ui-store";

describe("createUiStore", () => {
  it("toggles a source between expanded and collapsed", () => {
    const store = createUiStore();
    store.getState().toggleExpanded(10);
    expect(store.getState().expandedSources.has(10)).toBe(true);
    store.getState().toggleExpanded(10);
    expect(store.getState().expandedSources.has(10)).toBe(false);
  });

  it("drops expansion state for sources that no longer exist", () => {
    // After undo removes a source, its expansion entry must not linger.
    const store = createUiStore();
    store.getState().toggleExpanded(10);
    store.getState().pruneExpanded([20, 30]);
    expect(store.getState().expandedSources.size).toBe(0);
  });

  it("keeps the same expansion set when every source still exists", () => {
    const store = createUiStore();
    store.getState().toggleExpanded(10);
    const expandedSources = store.getState().expandedSources;

    store.getState().pruneExpanded([10, 20]);

    expect(store.getState().expandedSources).toBe(expandedSources);
  });

  it("replaces and clears the selected slots", () => {
    const store = createUiStore();

    store.getState().selectSlots([10, 20, 20]);
    expect(store.getState().selectedSlots).toEqual(new Set([10, 20]));

    store.getState().clearSelection();
    expect(store.getState().selectedSlots).toEqual(new Set());
  });

  it("keeps the same empty selection when clearing it again", () => {
    const store = createUiStore();
    const selectedSlots = store.getState().selectedSlots;

    store.getState().clearSelection();

    expect(store.getState().selectedSlots).toBe(selectedSlots);
  });

  it("prunes missing slots without replacing an unchanged selection", () => {
    const store = createUiStore();
    store.getState().selectSlots([10, 20]);
    const selectedSlots = store.getState().selectedSlots;

    store.getState().pruneSelected([10, 20, 30]);
    expect(store.getState().selectedSlots).toBe(selectedSlots);

    store.getState().pruneSelected([20, 30]);
    expect(store.getState().selectedSlots).toEqual(new Set([20]));
  });
});
