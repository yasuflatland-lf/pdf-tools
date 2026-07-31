import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "../App";
import type { PlanSnapshot } from "../bindings/PlanSnapshot";
import { usePlanStore } from "../store/plan-store";
import { useUiStore } from "../store/ui-store";

const { dragDrop, invoke, onDragDropEvent, unlisten } = vi.hoisted(() => {
  const dragDropState: {
    handler?: (event: { payload: unknown }) => unknown;
  } = {};
  const unlistenMock = vi.fn();
  const onDragDropEventMock = vi.fn((handler: (event: { payload: unknown }) => unknown) => {
    dragDropState.handler = handler;
    return Promise.resolve(unlistenMock);
  });

  return {
    dragDrop: dragDropState,
    invoke: vi.fn(),
    onDragDropEvent: onDragDropEventMock,
    unlisten: unlistenMock,
  };
});

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({ onDragDropEvent }),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const mountedRoots: Root[] = [];

async function renderApp(): Promise<HTMLElement> {
  const container = document.createElement("div");
  document.body.append(container);
  const root = createRoot(container);
  mountedRoots.push(root);

  await act(async () => {
    root.render(<App />);
  });

  return container;
}

describe("App", () => {
  beforeEach(() => {
    invoke.mockReset();
    onDragDropEvent.mockClear();
    unlisten.mockClear();
    dragDrop.handler = undefined;
    usePlanStore.getState().setSnapshot({
      slots: [],
      sources: [],
      can_undo: false,
      can_redo: false,
    });
    useUiStore.setState({ selectedSlots: new Set() });
  });

  afterEach(async () => {
    await act(async () => {
      for (const root of mountedRoots.splice(0)) {
        root.unmount();
      }
    });
    document.body.replaceChildren();
    useUiStore.setState({ selectedSlots: new Set() });
  });

  it("renders the empty document shell", async () => {
    const container = await renderApp();

    // The window title already names the app, so the shell starts at the
    // toolbar and spends none of its height repeating that name.
    expect(container.textContent).not.toContain("PDF Tools");
    expect(container.textContent).toContain("0 files");
    expect(container.textContent).toContain("0 pages");
    expect(onDragDropEvent).toHaveBeenCalledOnce();
    expect(invoke).not.toHaveBeenCalled();
  });

  it("highlights the content region while a drag is over the window", async () => {
    const container = await renderApp();

    await act(async () => {
      await dragDrop.handler?.({ payload: { type: "enter", paths: [], position: { x: 0, y: 0 } } });
    });
    expect(container.querySelector(".border-sky-400")).not.toBeNull();

    await act(async () => {
      await dragDrop.handler?.({ payload: { type: "leave" } });
    });
    expect(container.querySelector(".border-sky-400")).toBeNull();
  });

  it("keeps the shell usable when the command fails", async () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    invoke.mockRejectedValue("PDFium engine is unavailable");
    const container = await renderApp();

    await act(async () => {
      await dragDrop.handler?.({
        payload: { type: "drop", paths: ["/documents/report.pdf"], position: { x: 0, y: 0 } },
      });
    });

    expect(container.textContent).toContain("0 files");
    expect(consoleError).toHaveBeenCalled();
    consoleError.mockRestore();
  });

  it("shows a source returned after a file is dropped", async () => {
    invoke.mockResolvedValue({
      slots: [
        { id: 1, source: 10, page: 0 },
        { id: 2, source: 10, page: 1 },
      ],
      sources: [
        {
          id: 10,
          path: "/documents/report.pdf",
          file_name: "report.pdf",
          kind: "pdf",
          grouping: "grouped",
          page_count: 2,
          status: { kind: "ready" },
        },
      ],
    });
    const container = await renderApp();

    await act(async () => {
      await dragDrop.handler?.({
        payload: {
          type: "drop",
          paths: ["/documents/report.pdf"],
          position: { x: 20, y: 30 },
        },
      });
    });

    expect(invoke).toHaveBeenCalledWith("add_sources", {
      paths: ["/documents/report.pdf"],
    });
    expect(container.textContent).toContain("report.pdf");
    expect(container.textContent).toContain("1 file");
    expect(container.textContent).toContain("2 pages");
  });

  it("removes the selected slots with Delete", async () => {
    const loaded: PlanSnapshot = {
      slots: [
        { id: 1, source: 10, page: 0 },
        { id: 2, source: 10, page: 1 },
      ],
      sources: [
        {
          id: 10,
          path: "/documents/report.pdf",
          file_name: "report.pdf",
          kind: "pdf",
          grouping: "grouped",
          page_count: 2,
          status: { kind: "ready" as const },
        },
      ],
      can_undo: false,
      can_redo: false,
    };
    const removed = { ...loaded, slots: [loaded.slots[0]], can_undo: true };
    usePlanStore.getState().setSnapshot(loaded);
    useUiStore.setState({ selectedSlots: new Set([2]) });
    invoke.mockImplementation((command: string) =>
      Promise.resolve(command === "remove_slots" ? removed : new Uint8Array([1, 2, 3])),
    );
    await renderApp();

    await act(async () => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", {
          bubbles: true,
          cancelable: true,
          key: "Delete",
        }),
      );
      await Promise.resolve();
    });

    expect(invoke).toHaveBeenCalledWith("remove_slots", { slotIds: [2] });
    expect(usePlanStore.getState().slots).toEqual([loaded.slots[0]]);
  });

  it("does not remove slots while a text input has focus", async () => {
    usePlanStore.getState().setSnapshot({
      slots: [],
      sources: [],
      can_undo: false,
      can_redo: false,
    });
    useUiStore.setState({ selectedSlots: new Set([2]) });
    const input = document.createElement("input");
    document.body.append(input);
    await renderApp();
    invoke.mockClear();
    input.focus();

    await act(async () => {
      input.dispatchEvent(
        new KeyboardEvent("keydown", {
          bubbles: true,
          cancelable: true,
          key: "Delete",
        }),
      );
      await Promise.resolve();
    });

    expect(invoke).not.toHaveBeenCalled();
  });

  it("selects every slot with the platform modifier and A, and clears it with Escape", async () => {
    usePlanStore.getState().setSnapshot({
      slots: [
        { id: 1, source: 10, page: 0 },
        { id: 2, source: 10, page: 1 },
      ],
      sources: [],
      can_undo: false,
      can_redo: false,
    });
    await renderApp();

    await act(async () => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { bubbles: true, cancelable: true, ctrlKey: true, key: "a" }),
      );
    });
    expect(useUiStore.getState().selectedSlots).toEqual(new Set([1, 2]));

    await act(async () => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { bubbles: true, cancelable: true, key: "Escape" }),
      );
    });
    expect(useUiStore.getState().selectedSlots).toEqual(new Set());
  });

  it("keeps a source that cannot be merged on screen with its reason", async () => {
    invoke.mockResolvedValue({
      slots: [{ id: 1, source: 10, page: 0 }],
      sources: [
        {
          id: 10,
          path: "/documents/photo.png",
          file_name: "photo.png",
          kind: "image",
          grouping: "grouped",
          page_count: 1,
          status: { kind: "ready" },
        },
        {
          id: 11,
          path: "/documents/locked.pdf",
          file_name: "locked.pdf",
          kind: "pdf",
          grouping: "grouped",
          page_count: 0,
          status: { kind: "encrypted" },
        },
        {
          id: 12,
          path: "/documents/damaged.pdf",
          file_name: "damaged.pdf",
          kind: "pdf",
          grouping: "grouped",
          page_count: 0,
          status: { kind: "unreadable", reason: "broken xref" },
        },
      ],
      can_undo: true,
      can_redo: false,
    });
    const container = await renderApp();

    await act(async () => {
      await dragDrop.handler?.({
        payload: {
          type: "drop",
          paths: ["/documents/photo.png", "/documents/locked.pdf", "/documents/damaged.pdf"],
          position: { x: 20, y: 30 },
        },
      });
    });

    const unusable = container.querySelector('[aria-label="Unusable source files"]');
    expect(unusable?.textContent).toContain("locked.pdf");
    expect(unusable?.textContent).toMatch(/Password protected/);
    expect(unusable?.textContent).toContain("damaged.pdf");
    expect(unusable?.textContent).toContain("broken xref");
    // The readable source keeps its own card and is not dimmed with the others.
    expect(unusable?.textContent).not.toContain("photo.png");
    expect(container.querySelectorAll(".opacity-60")).toHaveLength(2);
  });
});
