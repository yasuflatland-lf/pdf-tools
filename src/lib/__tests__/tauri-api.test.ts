import { beforeEach, describe, expect, it, vi } from "vitest";
import type { PlanSnapshot } from "../../bindings/PlanSnapshot";
import { addSources } from "../tauri-api";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));

describe("addSources", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("returns the snapshot produced by the add_sources command", async () => {
    const paths = ["/documents/first.pdf", "/images/page.png"];
    const snapshot: PlanSnapshot = {
      slots: [{ id: 1, source: 10, page: 0 }],
      sources: [],
      can_undo: false,
      can_redo: false,
    };
    invoke.mockResolvedValue(snapshot);

    await expect(addSources(paths)).resolves.toBe(snapshot);
    expect(invoke).toHaveBeenCalledWith("add_sources", { paths });
  });
});
