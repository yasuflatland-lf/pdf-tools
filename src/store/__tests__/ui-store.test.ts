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
});
