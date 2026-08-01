import { describe, expect, it } from "vitest";
import type { DropGroup } from "../drop-position";
import { computeDropTarget } from "../drop-position";

function g(start: number, pageCount: number): DropGroup {
  return { start, pageCount };
}

describe("computeDropTarget", () => {
  it("translates a group drag into the slot range the backend expects", () => {
    // groups: [A(0..2), B(2..3), C(3..5)] -- drag A after B
    const groups = [g(0, 2), g(2, 1), g(3, 2)];
    expect(computeDropTarget(groups, 0, 1)).toEqual({ fromStart: 0, fromEnd: 2, to: 1 });
  });

  it("returns a no-op when a group is dropped on itself", () => {
    const groups = [g(0, 2), g(2, 1)];
    expect(computeDropTarget(groups, 0, 0)).toBeNull();
  });

  it("handles dropping a group at the very end", () => {
    const groups = [g(0, 2), g(2, 1), g(3, 2)];
    expect(computeDropTarget(groups, 0, 2)).toEqual({ fromStart: 0, fromEnd: 2, to: 3 });
  });

  it("handles dragging a group backward", () => {
    const groups = [g(0, 2), g(2, 1), g(3, 2)];
    expect(computeDropTarget(groups, 2, 0)).toEqual({ fromStart: 3, fromEnd: 5, to: 0 });
  });

  it.each([
    [-1, 0],
    [0, -1],
    [2, 0],
    [0, 2],
  ])("returns null for out-of-range indices (%i, %i)", (activeIndex, overIndex) => {
    const groups = [g(0, 2), g(2, 1)];
    expect(computeDropTarget(groups, activeIndex, overIndex)).toBeNull();
  });

  it("returns null when the active group covers no slots", () => {
    const groups = [g(0, 0), g(0, 1)];
    expect(computeDropTarget(groups, 0, 1)).toBeNull();
  });
});
