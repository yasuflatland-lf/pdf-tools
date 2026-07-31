import { beforeEach, describe, expect, it, vi } from "vitest";
import type { PlanSnapshot } from "../../bindings/PlanSnapshot";
import { addSources, reorder } from "../tauri-api";

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
});
