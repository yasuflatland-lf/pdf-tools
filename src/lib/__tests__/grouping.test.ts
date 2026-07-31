import { describe, expect, it } from "vitest";
import type { PageSlotDto } from "../../bindings/PageSlotDto";
import type { SourceFileDto } from "../../bindings/SourceFileDto";
import { groupContiguous } from "../grouping";

function slot(id: number, sourceId: number, page: number): PageSlotDto {
  return { id, source: sourceId, page, rotation: 0 };
}

function source(id: number, grouping: SourceFileDto["grouping"]): SourceFileDto {
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

describe("groupContiguous", () => {
  it("collapses contiguous slots from the same grouped source into one card", () => {
    const groups = groupContiguous(
      [slot(1, 10, 0), slot(2, 10, 1), slot(3, 20, 0)],
      [source(10, "grouped"), source(20, "grouped")],
    );

    expect(groups).toHaveLength(2);
    expect(groups[0]?.slotIds).toEqual([1, 2]);
  });

  it("never collapses slots from an ungrouped source", () => {
    const groups = groupContiguous([slot(1, 10, 0), slot(2, 10, 1)], [source(10, "ungrouped")]);

    expect(groups).toHaveLength(2);
  });

  it("shows the number of pages a group actually contains", () => {
    const groups = groupContiguous([slot(1, 10, 0), slot(2, 10, 2)], [source(10, "grouped")]);

    expect(groups[0]?.pageCount).toBe(2);
  });

  it("keeps noncontiguous runs from the same source separate", () => {
    const groups = groupContiguous(
      [slot(1, 10, 0), slot(2, 20, 0), slot(3, 10, 1)],
      [source(10, "grouped"), source(20, "grouped")],
    );

    expect(groups.map((group) => group.slotIds)).toEqual([[1], [2], [3]]);
  });

  it("creates one group per slot when the source is unknown", () => {
    const groups = groupContiguous([slot(1, 99, 0), slot(2, 99, 1)], []);

    expect(groups).toHaveLength(2);
  });
});
