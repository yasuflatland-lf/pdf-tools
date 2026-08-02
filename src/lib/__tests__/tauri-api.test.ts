import { beforeEach, describe, expect, it, vi } from "vitest";
import type { PlanSnapshot } from "../../bindings/PlanSnapshot";
import { addSources, removeSource, reorder, rotateSlots } from "../tauri-api";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));

describe("tauri-api", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("returns the snapshot produced by the add_sources command", async () => {
    const paths = ["/documents/first.pdf", "/images/page.png"];
    const snapshot: PlanSnapshot = {
      slots: [{ id: 1, source: 10, page: 0, rotation: 0 }],
      sources: [],
      can_undo: false,
      can_redo: false,
    };
    invoke.mockResolvedValue(snapshot);

    await expect(addSources(paths)).resolves.toBe(snapshot);
    expect(invoke).toHaveBeenCalledWith("add_sources", { paths });
  });

  it("passes the slot range and destination to the reorder command", async () => {
    const snapshot: PlanSnapshot = {
      slots: [],
      sources: [],
      can_undo: true,
      can_redo: false,
    };
    invoke.mockResolvedValue(snapshot);

    await expect(reorder(0, 2, 3)).resolves.toBe(snapshot);
    expect(invoke).toHaveBeenCalledWith("reorder", {
      fromStart: 0,
      fromEnd: 2,
      to: 3,
    });
  });

  it("passes the camel-cased source id to the remove_source command", async () => {
    const snapshot: PlanSnapshot = {
      slots: [],
      sources: [],
      can_undo: true,
      can_redo: false,
    };
    invoke.mockResolvedValue(snapshot);

    await expect(removeSource(7)).resolves.toBe(snapshot);
    expect(invoke).toHaveBeenCalledWith("remove_source", { sourceId: 7 });
  });

  it("passes camel-cased slot ids and the delta to the rotate_slots command", async () => {
    const snapshot: PlanSnapshot = {
      slots: [],
      sources: [],
      can_undo: true,
      can_redo: false,
    };
    invoke.mockResolvedValue(snapshot);

    await expect(rotateSlots([2, 4], -1)).resolves.toBe(snapshot);
    expect(invoke).toHaveBeenCalledWith("rotate_slots", { slotIds: [2, 4], delta: -1 });
  });
});
