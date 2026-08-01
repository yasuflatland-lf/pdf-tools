/** A draggable unit in the grid: a contiguous run of plan slots. */
export interface DropGroup {
  /** Index of the group's first slot in the plan. */
  start: number;
  /** How many consecutive slots the group covers. */
  pageCount: number;
}

export interface DropTarget {
  fromStart: number;
  fromEnd: number;
  to: number;
}

/**
 * Translates card indices into the backend's slot-based reorder coordinates.
 *
 * `to` is an insertion index in the slot sequence that remains after the
 * half-open `from` range has been lifted out.
 */
export function computeDropTarget(
  groups: readonly DropGroup[],
  activeGroupIndex: number,
  overGroupIndex: number,
): DropTarget | null {
  if (
    activeGroupIndex === overGroupIndex ||
    activeGroupIndex < 0 ||
    activeGroupIndex >= groups.length ||
    overGroupIndex < 0 ||
    overGroupIndex >= groups.length
  ) {
    return null;
  }

  const active = groups[activeGroupIndex];
  if (active.pageCount <= 0) {
    return null;
  }

  const over = groups[overGroupIndex];
  const to =
    overGroupIndex < activeGroupIndex ? over.start : over.start + over.pageCount - active.pageCount;

  return {
    fromStart: active.start,
    fromEnd: active.start + active.pageCount,
    to,
  };
}
