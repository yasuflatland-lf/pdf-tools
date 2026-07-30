import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ComposeProgressDto } from "../../bindings/ComposeProgressDto";
import type { MergeReportDto } from "../../bindings/MergeReportDto";
import { usePlanStore } from "../../store/plan-store";
import { Toolbar } from "../Toolbar";

const mocks = vi.hoisted(() => ({
  compose: vi.fn(),
  defaultOutputDir: vi.fn(),
  joinPath: vi.fn((dir: string, name: string) => `${dir}/${name}`),
  onComposeProgress: vi.fn(),
  parentDir: vi.fn((path: string) => path.slice(0, path.lastIndexOf("/"))),
  progress: {} as { handler?: (progress: ComposeProgressDto) => void },
  rememberOutputDir: vi.fn(),
  redo: vi.fn(),
  revealItemInDir: vi.fn(),
  save: vi.fn(),
  undo: vi.fn(),
  unlisten: vi.fn(),
}));

vi.mock("../../lib/tauri-api", () => ({
  compose: mocks.compose,
  onComposeProgress: mocks.onComposeProgress,
  redo: mocks.redo,
  undo: mocks.undo,
}));
vi.mock("../../lib/output-dir", () => ({
  defaultOutputDir: mocks.defaultOutputDir,
  joinPath: mocks.joinPath,
  parentDir: mocks.parentDir,
  rememberOutputDir: mocks.rememberOutputDir,
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ save: mocks.save }));
vi.mock("@tauri-apps/plugin-opener", () => ({ revealItemInDir: mocks.revealItemInDir }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const mountedRoots: Root[] = [];

async function renderToolbar(): Promise<HTMLElement> {
  const container = document.createElement("div");
  document.body.append(container);
  const root = createRoot(container);
  mountedRoots.push(root);

  await act(async () => {
    root.render(<Toolbar />);
  });

  return container;
}

function getButton(container: HTMLElement, name: string): HTMLButtonElement {
  const button = Array.from(container.querySelectorAll("button")).find(
    (candidate) => candidate.textContent === name,
  );
  if (!button) {
    throw new Error(`Button "${name}" was not found`);
  }
  return button;
}

function loadOneSlot(): void {
  usePlanStore.getState().setSnapshot({
    slots: [{ id: 1, source: 10, page: 0 }],
    sources: [],
    can_undo: false,
    can_redo: false,
  });
}

function deferred<T>(): {
  promise: Promise<T>;
  resolve: (value: T) => void;
  reject: (reason: unknown) => void;
} {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

/** The macOS redo chord, which reports an upper-case `key` because of the shift. */
function redoShortcut(): KeyboardEvent {
  return new KeyboardEvent("keydown", {
    bubbles: true,
    cancelable: true,
    key: "Z",
    metaKey: true,
    shiftKey: true,
  });
}

async function click(button: HTMLButtonElement): Promise<void> {
  await act(async () => {
    button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await Promise.resolve();
  });
}

describe("Toolbar", () => {
  beforeEach(() => {
    mocks.compose.mockReset();
    mocks.defaultOutputDir.mockReset();
    mocks.joinPath.mockClear();
    mocks.onComposeProgress.mockReset();
    mocks.parentDir.mockClear();
    mocks.rememberOutputDir.mockReset();
    mocks.revealItemInDir.mockReset();
    mocks.save.mockReset();
    mocks.unlisten.mockReset();
    mocks.progress.handler = undefined;
    mocks.redo.mockReset();
    mocks.undo.mockReset();

    mocks.defaultOutputDir.mockResolvedValue("/Users/me/Downloads");
    mocks.onComposeProgress.mockImplementation(
      async (handler: (progress: ComposeProgressDto) => void) => {
        mocks.progress.handler = handler;
        return mocks.unlisten;
      },
    );
    usePlanStore
      .getState()
      .setSnapshot({ slots: [], sources: [], can_undo: false, can_redo: false });
  });

  afterEach(async () => {
    await act(async () => {
      for (const root of mountedRoots.splice(0)) {
        root.unmount();
      }
    });
    document.body.replaceChildren();
    usePlanStore
      .getState()
      .setSnapshot({ slots: [], sources: [], can_undo: false, can_redo: false });
    vi.restoreAllMocks();
  });

  it("enables Merge only when the plan has pages", async () => {
    const container = await renderToolbar();

    expect(getButton(container, "Merge").disabled).toBe(true);

    await act(async () => {
      loadOneSlot();
    });

    expect(getButton(container, "Merge").disabled).toBe(false);
  });

  it("disables Undo and Redo when the snapshot says they are unavailable", async () => {
    const container = await renderToolbar();

    expect(getButton(container, "Undo").disabled).toBe(true);
    expect(getButton(container, "Redo").disabled).toBe(true);
  });

  it("stores the snapshot returned by Undo", async () => {
    usePlanStore.getState().setSnapshot({
      slots: [{ id: 1, source: 10, page: 0 }],
      sources: [],
      can_undo: true,
      can_redo: false,
    });
    mocks.undo.mockResolvedValue({
      slots: [{ id: 2, source: 20, page: 0 }],
      sources: [],
      can_undo: false,
      can_redo: true,
    });
    const container = await renderToolbar();

    await click(getButton(container, "Undo"));

    expect(mocks.undo).toHaveBeenCalledOnce();
    expect(usePlanStore.getState().slots).toEqual([{ id: 2, source: 20, page: 0 }]);
    expect(usePlanStore.getState().canRedo).toBe(true);
  });

  it("stores the snapshot returned by the Ctrl+Z shortcut", async () => {
    usePlanStore.getState().setSnapshot({
      slots: [{ id: 1, source: 10, page: 0 }],
      sources: [],
      can_undo: true,
      can_redo: false,
    });
    mocks.undo.mockResolvedValue({
      slots: [],
      sources: [],
      can_undo: false,
      can_redo: true,
    });
    await renderToolbar();
    const event = new KeyboardEvent("keydown", {
      bubbles: true,
      cancelable: true,
      ctrlKey: true,
      key: "z",
    });

    await act(async () => {
      window.dispatchEvent(event);
      await Promise.resolve();
    });

    expect(event.defaultPrevented).toBe(true);
    expect(mocks.undo).toHaveBeenCalledOnce();
    expect(usePlanStore.getState().slots).toEqual([]);
    expect(usePlanStore.getState().canRedo).toBe(true);
  });

  it("redoes on the shifted shortcut and ignores it when there is nothing to redo", async () => {
    usePlanStore
      .getState()
      .setSnapshot({ slots: [], sources: [], can_undo: true, can_redo: false });
    mocks.redo.mockResolvedValue({
      slots: [{ id: 3, source: 10, page: 2 }],
      sources: [],
      can_undo: true,
      can_redo: false,
    });
    await renderToolbar();

    const ignored = redoShortcut();
    await act(async () => {
      window.dispatchEvent(ignored);
      await Promise.resolve();
    });
    expect(mocks.redo).not.toHaveBeenCalled();
    expect(ignored.defaultPrevented).toBe(false);
    // The shifted shortcut must never fall through to an undo.
    expect(mocks.undo).not.toHaveBeenCalled();

    await act(async () => {
      usePlanStore
        .getState()
        .setSnapshot({ slots: [], sources: [], can_undo: false, can_redo: true });
    });
    await act(async () => {
      window.dispatchEvent(redoShortcut());
      await Promise.resolve();
    });

    expect(mocks.redo).toHaveBeenCalledOnce();
    expect(usePlanStore.getState().slots).toEqual([{ id: 3, source: 10, page: 2 }]);
  });

  it("logs a failed undo and leaves the plan untouched", async () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    usePlanStore.getState().setSnapshot({
      slots: [{ id: 1, source: 10, page: 0 }],
      sources: [],
      can_undo: true,
      can_redo: false,
    });
    mocks.undo.mockRejectedValue(new Error("history unavailable"));
    const container = await renderToolbar();

    await click(getButton(container, "Undo"));

    expect(consoleError).toHaveBeenCalledWith("undo failed", expect.any(Error));
    expect(usePlanStore.getState().slots).toEqual([{ id: 1, source: 10, page: 0 }]);
    consoleError.mockRestore();
  });

  it("opens the save dialog with a merged PDF default path", async () => {
    loadOneSlot();
    mocks.save.mockResolvedValue(null);
    const container = await renderToolbar();

    await click(getButton(container, "Merge"));

    expect(mocks.save).toHaveBeenCalledWith({
      defaultPath: "/Users/me/Downloads/merged.pdf",
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });
  });

  it("does not compose after the save dialog is cancelled", async () => {
    loadOneSlot();
    mocks.save.mockResolvedValue(null);
    const container = await renderToolbar();

    await click(getButton(container, "Merge"));

    expect(mocks.compose).not.toHaveBeenCalled();
    expect(getButton(container, "Merge").disabled).toBe(false);
  });

  it("reports progress, completes the merge, and reveals its output", async () => {
    const outputPath = "/Users/me/Documents/book.pdf";
    const pending = deferred<MergeReportDto>();
    loadOneSlot();
    mocks.save.mockResolvedValue(outputPath);
    mocks.compose.mockReturnValue(pending.promise);
    const container = await renderToolbar();

    await click(getButton(container, "Merge"));

    expect(mocks.compose).toHaveBeenCalledWith(outputPath);
    expect(mocks.rememberOutputDir).toHaveBeenCalledWith("/Users/me/Documents");
    expect(getButton(container, "Merge").disabled).toBe(true);

    await act(async () => {
      mocks.progress.handler?.({ done: 1, total: 2 });
    });
    expect(container.querySelector('[role="progressbar"]')?.getAttribute("aria-valuenow")).toBe(
      "50",
    );

    await act(async () => {
      pending.resolve({ page_count: 3, bytes_written: 1024 });
      await pending.promise;
    });

    expect(getButton(container, "Merge").disabled).toBe(false);
    expect(container.textContent).toContain("book.pdf");
    expect(container.textContent).toContain("3 pages");
    expect(mocks.unlisten).toHaveBeenCalledOnce();

    await click(getButton(container, "Show in folder"));
    expect(mocks.revealItemInDir).toHaveBeenCalledWith(outputPath);
  });

  it("names the failing file in a dialog and keeps the marker after it is closed", async () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    usePlanStore.getState().setSnapshot({
      slots: [{ id: 1, source: 10, page: 0 }],
      sources: [
        {
          id: 10,
          path: "/documents/report.pdf",
          file_name: "report.pdf",
          kind: "pdf",
          grouping: "grouped",
          page_count: 1,
          status: { kind: "ready" },
        },
      ],
      can_undo: false,
      can_redo: false,
    });
    mocks.save.mockResolvedValue("/Users/me/Documents/book.pdf");
    mocks.compose.mockRejectedValue("failed to read the PDF at /documents/report.pdf: broken xref");
    const container = await renderToolbar();

    await click(getButton(container, "Merge"));

    const dialog = container.querySelector('[role="alertdialog"]');
    expect(dialog?.textContent).toContain("report.pdf");
    expect(dialog?.textContent).toContain("broken xref");

    await click(getButton(container, "閉じる"));

    expect(container.querySelector('[role="alertdialog"]')).toBeNull();
    expect(container.textContent).toContain("Merge failed");
    consoleError.mockRestore();
  });

  it("shows a failure, re-enables Merge, and removes the progress listener", async () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    loadOneSlot();
    mocks.save.mockResolvedValue("/Users/me/Documents/book.pdf");
    mocks.compose.mockRejectedValue(new Error("merge unavailable"));
    const container = await renderToolbar();

    await click(getButton(container, "Merge"));

    expect(container.textContent).toContain("Merge failed");
    expect(getButton(container, "Merge").disabled).toBe(false);
    expect(mocks.unlisten).toHaveBeenCalledOnce();
    expect(consoleError).toHaveBeenCalledWith("compose failed", expect.any(Error));
  });
});
