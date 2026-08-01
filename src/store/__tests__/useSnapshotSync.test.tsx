import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { SourceFileDto } from "../../bindings/SourceFileDto";
import { usePlanStore } from "../plan-store";
import { useSnapshotSync } from "../useSnapshotSync";
import { useUiStore } from "../ui-store";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const mountedRoots: Root[] = [];

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

async function mountSync(child?: React.ReactNode): Promise<void> {
  const container = document.createElement("div");
  document.body.append(container);
  const root = createRoot(container);
  mountedRoots.push(root);

  function SyncHarness() {
    useSnapshotSync();
    return child;
  }

  await act(async () => {
    root.render(<SyncHarness />);
  });
}

describe("useSnapshotSync", () => {
  beforeEach(() => {
    usePlanStore.setState({ slots: [], sources: [], canUndo: false, canRedo: false });
    useUiStore.setState({
      expandedSources: new Set(),
      selectedSlots: new Set(),
      viewMode: "grid",
      modalOpen: false,
    });
  });

  afterEach(async () => {
    await act(async () => {
      for (const root of mountedRoots.splice(0)) {
        root.unmount();
      }
    });
    document.body.replaceChildren();
  });

  it("collapses an expanded source removed by an undo snapshot", async () => {
    usePlanStore.setState({ sources: [source(10, "grouped")] });
    useUiStore.getState().toggleExpanded(10);
    await mountSync();

    act(() => {
      usePlanStore.getState().setSnapshot({
        slots: [],
        sources: [],
        can_undo: false,
        can_redo: true,
      });
    });

    expect(useUiStore.getState().expandedSources.size).toBe(0);
  });

  it("drops selected slots that the snapshot no longer contains", async () => {
    useUiStore.getState().selectSlots([1, 2]);
    await mountSync();

    act(() => {
      usePlanStore.getState().setSnapshot({
        slots: [{ id: 2, source: 10, page: 1, rotation: 0 }],
        sources: [source(10, "grouped")],
        can_undo: true,
        can_redo: false,
      });
    });

    expect(useUiStore.getState().selectedSlots).toEqual(new Set([2]));
  });

  it("re-collapses a source that is grouped again after being ungrouped", async () => {
    usePlanStore.setState({ sources: [source(10, "grouped")] });
    useUiStore.getState().toggleExpanded(10);
    await mountSync();

    act(() => {
      usePlanStore.getState().setSnapshot({
        slots: [],
        sources: [source(10, "ungrouped")],
        can_undo: true,
        can_redo: false,
      });
    });
    expect(useUiStore.getState().expandedSources.size).toBe(0);

    act(() => {
      usePlanStore.getState().setSnapshot({
        slots: [],
        sources: [source(10, "grouped")],
        can_undo: true,
        can_redo: false,
      });
    });

    expect(useUiStore.getState().expandedSources.has(10)).toBe(false);
  });

  it("renders the snapshot and pruned UI state in one consistent pass", async () => {
    usePlanStore.setState({ sources: [source(10, "grouped")] });
    useUiStore.getState().toggleExpanded(10);
    const renders: { sourcePresent: boolean; sourceExpanded: boolean }[] = [];

    function StateProbe() {
      const sourcePresent = usePlanStore((state) => state.sources.some(({ id }) => id === 10));
      const sourceExpanded = useUiStore((state) => state.expandedSources.has(10));
      renders.push({ sourcePresent, sourceExpanded });
      return null;
    }

    await mountSync(<StateProbe />);
    expect(renders).toEqual([{ sourcePresent: true, sourceExpanded: true }]);

    act(() => {
      usePlanStore.getState().setSnapshot({
        slots: [],
        sources: [],
        can_undo: false,
        can_redo: true,
      });
    });

    expect(renders).toEqual([
      { sourcePresent: true, sourceExpanded: true },
      { sourcePresent: false, sourceExpanded: false },
    ]);
  });
});
