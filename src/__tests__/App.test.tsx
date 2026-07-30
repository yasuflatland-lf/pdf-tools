import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "../App";
import { usePlanStore } from "../store/plan-store";

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
  });

  afterEach(async () => {
    await act(async () => {
      for (const root of mountedRoots.splice(0)) {
        root.unmount();
      }
    });
    document.body.replaceChildren();
  });

  it("renders the empty document shell", async () => {
    const container = await renderApp();

    expect(container.textContent).toContain("PDF Tools");
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
});
