import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useUiStore } from "../../store/ui-store";
import { DropZone } from "../DropZone";

const { addSources, dragDrop, expandPaths, onDragDropEvent } = vi.hoisted(() => {
  const dragDropState: { handler?: (event: { payload: unknown }) => unknown } = {};

  return {
    addSources: vi.fn(),
    dragDrop: dragDropState,
    expandPaths: vi.fn(),
    onDragDropEvent: vi.fn((handler: (event: { payload: unknown }) => unknown) => {
      dragDropState.handler = handler;
      return Promise.resolve(() => {});
    }),
  };
});

vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({ onDragDropEvent }),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ ask: vi.fn(), open: vi.fn() }));
vi.mock("../../lib/tauri-api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../lib/tauri-api")>()),
  addSources,
  expandPaths,
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const mountedRoots: Root[] = [];

async function renderDropZone(): Promise<void> {
  const container = document.createElement("div");
  document.body.append(container);
  const root = createRoot(container);
  mountedRoots.push(root);

  await act(async () => {
    root.render(<DropZone>{null}</DropZone>);
  });
}

async function drop(paths: string[]): Promise<void> {
  await act(async () => {
    // `position` is part of Tauri's drop payload and `pnpm build` typechecks
    // these files, so it is supplied even though the listener ignores it.
    await dragDrop.handler?.({
      payload: { type: "drop", paths, position: { x: 0, y: 0 } },
    });
  });
}

describe("DropZone", () => {
  beforeEach(() => {
    addSources.mockReset();
    addSources.mockResolvedValue({
      slots: [],
      sources: [],
      can_undo: false,
      can_redo: false,
    });
    expandPaths.mockReset();
    useUiStore.setState({ isIngesting: false, sourceNotice: null });
  });

  afterEach(async () => {
    await act(async () => {
      for (const root of mountedRoots.splice(0)) {
        root.unmount();
      }
    });
    document.body.replaceChildren();
    useUiStore.setState({ isIngesting: false, sourceNotice: null });
    vi.restoreAllMocks();
  });

  it("expands a dropped folder before adding it", async () => {
    expandPaths.mockResolvedValue(["/scans/a.pdf", "/scans/b.pdf"]);
    await renderDropZone();

    await drop(["/scans"]);

    expect(expandPaths).toHaveBeenCalledWith(["/scans"]);
    expect(addSources).toHaveBeenCalledWith(["/scans/a.pdf", "/scans/b.pdf"]);
  });

  it("names a dropped folder that held nothing supported", async () => {
    expandPaths.mockResolvedValue([]);
    await renderDropZone();

    await drop(["/home/me/Scans"]);

    expect(useUiStore.getState().sourceNotice).toBe('No PDFs or images found in "Scans".');
    expect(addSources).not.toHaveBeenCalled();
  });

  it("ignores a drop that carried no paths", async () => {
    await renderDropZone();

    await drop([]);

    expect(expandPaths).not.toHaveBeenCalled();
  });
});
