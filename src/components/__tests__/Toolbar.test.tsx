import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ComposeProgressDto } from "../../bindings/ComposeProgressDto";
import type { MergeReportDto } from "../../bindings/MergeReportDto";
import { shortcutHint } from "../../lib/keyboard";
import { usePlanStore } from "../../store/plan-store";
import { useUiStore } from "../../store/ui-store";
import { AppShell } from "../AppShell";
import { Toolbar } from "../Toolbar";

const mocks = vi.hoisted(() => ({
  addSources: vi.fn(),
  ask: vi.fn(),
  compose: vi.fn(),
  defaultOutputDir: vi.fn(),
  expandPaths: vi.fn(),
  joinPath: vi.fn((dir: string, name: string) => `${dir}/${name}`),
  onComposeProgress: vi.fn(),
  onDragDropEvent: vi.fn(() => Promise.resolve(() => {})),
  open: vi.fn(),
  parentDir: vi.fn((path: string) => path.slice(0, path.lastIndexOf("/"))),
  progress: {} as { handler?: (progress: ComposeProgressDto) => void },
  rasterizeSlot: vi.fn(),
  rememberOutputDir: vi.fn(),
  redo: vi.fn(),
  removeSlots: vi.fn(),
  reorder: vi.fn(),
  revealItemInDir: vi.fn(),
  rotateSlots: vi.fn(),
  save: vi.fn(),
  supportedExtensions: vi.fn(),
  undo: vi.fn(),
  unlisten: vi.fn(),
}));

vi.mock("../../lib/tauri-api", () => ({
  addSources: mocks.addSources,
  compose: mocks.compose,
  expandPaths: mocks.expandPaths,
  onComposeProgress: mocks.onComposeProgress,
  rasterizeSlot: mocks.rasterizeSlot,
  redo: mocks.redo,
  removeSlots: mocks.removeSlots,
  reorder: mocks.reorder,
  rotateSlots: mocks.rotateSlots,
  supportedExtensions: mocks.supportedExtensions,
  undo: mocks.undo,
}));
vi.mock("../../lib/output-dir", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../lib/output-dir")>()),
  defaultOutputDir: mocks.defaultOutputDir,
  joinPath: mocks.joinPath,
  parentDir: mocks.parentDir,
  rememberOutputDir: mocks.rememberOutputDir,
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({
  ask: mocks.ask,
  open: mocks.open,
  save: mocks.save,
}));
vi.mock("@tauri-apps/plugin-opener", () => ({ revealItemInDir: mocks.revealItemInDir }));
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({ onDragDropEvent: mocks.onDragDropEvent }),
}));

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

async function renderShell(): Promise<HTMLElement> {
  const container = document.createElement("div");
  document.body.append(container);
  const root = createRoot(container);
  mountedRoots.push(root);

  await act(async () => {
    root.render(<AppShell />);
  });

  return container;
}

function getButton(container: HTMLElement, name: string): HTMLButtonElement {
  const button = Array.from(container.querySelectorAll("button")).find(
    (candidate) => candidate.getAttribute("aria-label") === name || candidate.textContent === name,
  );
  if (!button) {
    throw new Error(`Button "${name}" was not found`);
  }
  return button;
}

function loadOneSlot(): void {
  usePlanStore.getState().setSnapshot({
    slots: [{ id: 1, source: 10, page: 0, rotation: 0 }],
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

function undoShortcut(): KeyboardEvent {
  return new KeyboardEvent("keydown", {
    bubbles: true,
    cancelable: true,
    ctrlKey: true,
    key: "z",
  });
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

function rotateRightShortcut(): KeyboardEvent {
  return new KeyboardEvent("keydown", {
    bubbles: true,
    cancelable: true,
    key: "]",
    metaKey: true,
  });
}

async function click(button: HTMLButtonElement): Promise<void> {
  await act(async () => {
    button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await Promise.resolve();
  });
}

function keyDown(init: KeyboardEventInit): void {
  act(() => {
    window.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, ...init }));
  });
}

/** A key press that starts where the focus actually is, so it can be swallowed. */
function keyDownOn(target: HTMLElement, key: string): void {
  act(() => {
    target.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, cancelable: true, key }));
  });
}

describe("Toolbar", () => {
  beforeEach(() => {
    mocks.addSources.mockReset();
    mocks.ask.mockReset();
    mocks.compose.mockReset();
    mocks.defaultOutputDir.mockReset();
    mocks.expandPaths.mockReset();
    mocks.expandPaths.mockResolvedValue([]);
    mocks.joinPath.mockClear();
    mocks.onComposeProgress.mockReset();
    mocks.onDragDropEvent.mockClear();
    mocks.open.mockReset();
    mocks.open.mockResolvedValue(null);
    mocks.parentDir.mockClear();
    mocks.rememberOutputDir.mockReset();
    mocks.revealItemInDir.mockReset();
    mocks.save.mockReset();
    mocks.supportedExtensions.mockReset();
    mocks.supportedExtensions.mockResolvedValue(["pdf", "jpg", "jpeg", "png", "gif"]);
    mocks.unlisten.mockReset();
    mocks.progress.handler = undefined;
    mocks.rasterizeSlot.mockReset();
    mocks.redo.mockReset();
    mocks.removeSlots.mockReset();
    mocks.reorder.mockReset();
    mocks.rotateSlots.mockReset();
    mocks.undo.mockReset();

    mocks.defaultOutputDir.mockResolvedValue("/Users/me/Downloads");
    mocks.rasterizeSlot.mockResolvedValue(new Uint8Array([1, 2, 3]).buffer);
    mocks.onComposeProgress.mockImplementation(
      async (handler: (progress: ComposeProgressDto) => void) => {
        mocks.progress.handler = handler;
        return mocks.unlisten;
      },
    );
    usePlanStore
      .getState()
      .setSnapshot({ slots: [], sources: [], can_undo: false, can_redo: false });
    useUiStore.setState({ viewMode: "grid", modalOpen: false, selectedSlots: new Set() });
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
    useUiStore.setState({ viewMode: "grid", modalOpen: false, selectedSlots: new Set() });
    vi.restoreAllMocks();
  });

  async function openTheFailureDialog(): Promise<void> {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    loadOneSlot();
    usePlanStore.setState({ canUndo: true });
    mocks.save.mockResolvedValue("/Users/me/Documents/book.pdf");
    mocks.compose.mockRejectedValue(new Error("merge unavailable"));
    const container = await renderShell();

    await click(getButton(container, "Merge"));

    expect(container.querySelector('[role="alertdialog"]')).not.toBeNull();
    consoleError.mockRestore();
  }

  it("heads the toolbar with the application logo", async () => {
    const container = await renderToolbar();
    const logo = container.querySelector("img");

    expect(logo?.getAttribute("src")).toBe("/logo.svg");
    expect(logo?.getAttribute("alt")).toBe("PDF Tools");
  });

  it("opens the add menu and closes it on Escape, keeping the page selection", async () => {
    loadOneSlot();
    const container = await renderShell();
    const trigger = getButton(container, "Add files or a folder");
    act(() => {
      useUiStore.getState().selectSlots([1]);
    });

    expect(trigger.getAttribute("aria-expanded")).toBe("false");
    expect(container.querySelector('[role="menu"]')).toBeNull();

    await click(trigger);

    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    expect(container.querySelector('[role="menu"]')).not.toBeNull();
    expect(getButton(container, "Files…")).toBeDefined();
    expect(getButton(container, "Folder…")).toBeDefined();

    keyDownOn(trigger, "Escape");

    expect(container.querySelector('[role="menu"]')).toBeNull();
    expect(document.activeElement).toBe(trigger);
    // One key press, one effect: the Escape that closes the menu must not also
    // reach the shell's clear-selection shortcut behind it.
    expect(useUiStore.getState().selectedSlots.size).toBe(1);
  });

  it("moves between the menu items on the arrows without reaching the grid", async () => {
    loadOneSlot();
    const container = await renderShell();
    const trigger = getButton(container, "Add files or a folder");

    await click(trigger);
    keyDownOn(trigger, "ArrowDown");
    expect(document.activeElement).toBe(getButton(container, "Files…"));

    keyDownOn(getButton(container, "Files…"), "ArrowDown");
    expect(document.activeElement).toBe(getButton(container, "Folder…"));

    // The last item wraps back to the first rather than falling out of the menu.
    keyDownOn(getButton(container, "Folder…"), "ArrowDown");
    expect(document.activeElement).toBe(getButton(container, "Files…"));

    keyDownOn(getButton(container, "Files…"), "ArrowUp");
    expect(document.activeElement).toBe(getButton(container, "Folder…"));

    expect(container.querySelector('[role="menu"]')).not.toBeNull();
    // The grid moves the focus and drags the selection along with it, so an
    // untouched selection is how "the arrows reached nothing" is observed.
    expect(useUiStore.getState().selectedSlots.size).toBe(0);
  });

  it("closes the menu on a mousedown outside it, leaving the focus alone", async () => {
    const container = await renderToolbar();

    await click(getButton(container, "Add files or a folder"));
    const focused = document.activeElement;
    await act(async () => {
      document.body.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
    });

    expect(container.querySelector('[role="menu"]')).toBeNull();
    // The pointer is already taking the focus somewhere, so the menu must not
    // pull it back to the trigger the way Escape does.
    expect(document.activeElement).toBe(focused);
  });

  it("keeps the menu open on a mousedown inside it", async () => {
    const container = await renderToolbar();
    const trigger = getButton(container, "Add files or a folder");

    await click(trigger);
    // The trigger's own mousedown must not close the menu, or its click would
    // immediately toggle it back open and it could never be dismissed.
    await act(async () => {
      trigger.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
    });

    expect(container.querySelector('[role="menu"]')).not.toBeNull();
  });

  it("closes the menu after the Files… item runs", async () => {
    const container = await renderToolbar();

    await click(getButton(container, "Add files or a folder"));
    await click(getButton(container, "Files…"));

    expect(container.querySelector('[role="menu"]')).toBeNull();
    expect(mocks.open).toHaveBeenCalledWith({
      multiple: true,
      filters: [{ name: "PDFs and images", extensions: ["pdf", "jpg", "jpeg", "png", "gif"] }],
    });
  });

  it("closes the menu after the Folder… item runs", async () => {
    const container = await renderToolbar();

    await click(getButton(container, "Add files or a folder"));
    await click(getButton(container, "Folder…"));

    expect(container.querySelector('[role="menu"]')).toBeNull();
    expect(mocks.open).toHaveBeenCalledWith({ directory: true });
  });

  it("switches between grid and list views and reports the active mode", async () => {
    const container = await renderToolbar();
    const grid = getButton(container, "Grid view");
    const list = getButton(container, "List view");

    expect(grid.getAttribute("aria-pressed")).toBe("true");
    expect(list.getAttribute("aria-pressed")).toBe("false");

    await click(list);

    expect(useUiStore.getState().viewMode).toBe("list");
    expect(grid.getAttribute("aria-pressed")).toBe("false");
    expect(list.getAttribute("aria-pressed")).toBe("true");

    await click(grid);

    expect(useUiStore.getState().viewMode).toBe("grid");
    expect(grid.getAttribute("aria-pressed")).toBe("true");
    expect(list.getAttribute("aria-pressed")).toBe("false");
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

  it("enables rotation only while the selection is non-empty", async () => {
    const container = await renderToolbar();

    expect(getButton(container, "Rotate left").disabled).toBe(true);
    expect(getButton(container, "Rotate right").disabled).toBe(true);

    act(() => {
      useUiStore.getState().selectSlots([1]);
    });

    expect(getButton(container, "Rotate left").disabled).toBe(false);
    expect(getButton(container, "Rotate right").disabled).toBe(false);
  });

  it("rotates the current selection from the toolbar controls", async () => {
    mocks.rotateSlots.mockResolvedValue({
      slots: [],
      sources: [],
      can_undo: true,
      can_redo: false,
    });
    useUiStore.getState().selectSlots([2, 4]);
    const container = await renderToolbar();

    await click(getButton(container, "Rotate left"));
    await click(getButton(container, "Rotate right"));

    expect(mocks.rotateSlots.mock.calls).toEqual([
      [[2, 4], -1],
      [[2, 4], 1],
    ]);
  });

  it("rotates the current selection from the Cmd+] shortcut", async () => {
    loadOneSlot();
    useUiStore.getState().selectSlots([1]);
    mocks.rotateSlots.mockResolvedValue({
      slots: [{ id: 1, source: 10, page: 0, rotation: 1 }],
      sources: [],
      can_undo: true,
      can_redo: false,
    });
    await renderToolbar();
    const event = rotateRightShortcut();

    await act(async () => {
      window.dispatchEvent(event);
      await Promise.resolve();
    });

    expect(mocks.rotateSlots).toHaveBeenCalledOnce();
    expect(mocks.rotateSlots).toHaveBeenCalledWith([1], 1);
    expect(event.defaultPrevented).toBe(true);
  });

  it("ignores the Cmd+] shortcut when the selection is empty", async () => {
    loadOneSlot();
    await renderToolbar();
    const event = rotateRightShortcut();

    await act(async () => {
      window.dispatchEvent(event);
      await Promise.resolve();
    });

    expect(mocks.rotateSlots).not.toHaveBeenCalled();
    expect(event.defaultPrevented).toBe(false);
  });

  it("ignores the Cmd+] shortcut while merging", async () => {
    const pending = deferred<MergeReportDto>();
    loadOneSlot();
    useUiStore.getState().selectSlots([1]);
    mocks.save.mockResolvedValue("/Users/me/Documents/book.pdf");
    mocks.compose.mockReturnValue(pending.promise);
    const container = await renderToolbar();

    await click(getButton(container, "Merge"));
    const event = rotateRightShortcut();
    await act(async () => {
      window.dispatchEvent(event);
      await Promise.resolve();
    });

    expect(mocks.rotateSlots).not.toHaveBeenCalled();
    expect(event.defaultPrevented).toBe(false);

    await act(async () => {
      pending.resolve({ page_count: 1, bytes_written: 512 });
      await pending.promise;
    });
  });

  it("stores the snapshot returned by Undo", async () => {
    usePlanStore.getState().setSnapshot({
      slots: [{ id: 1, source: 10, page: 0, rotation: 0 }],
      sources: [],
      can_undo: true,
      can_redo: false,
    });
    mocks.undo.mockResolvedValue({
      slots: [{ id: 2, source: 20, page: 0, rotation: 0 }],
      sources: [],
      can_undo: false,
      can_redo: true,
    });
    const container = await renderToolbar();

    await click(getButton(container, "Undo"));

    expect(mocks.undo).toHaveBeenCalledOnce();
    expect(usePlanStore.getState().slots).toEqual([{ id: 2, source: 20, page: 0, rotation: 0 }]);
    expect(usePlanStore.getState().canRedo).toBe(true);
  });

  it("stores the snapshot returned by the Ctrl+Z shortcut", async () => {
    usePlanStore.getState().setSnapshot({
      slots: [{ id: 1, source: 10, page: 0, rotation: 0 }],
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
    const event = undoShortcut();

    await act(async () => {
      window.dispatchEvent(event);
      await Promise.resolve();
    });

    expect(event.defaultPrevented).toBe(true);
    expect(mocks.undo).toHaveBeenCalledOnce();
    expect(usePlanStore.getState().slots).toEqual([]);
    expect(usePlanStore.getState().canRedo).toBe(true);
  });

  it("ignores the undo shortcut when there is nothing to undo", async () => {
    usePlanStore
      .getState()
      .setSnapshot({ slots: [], sources: [], can_undo: false, can_redo: true });
    await renderToolbar();

    const ignored = undoShortcut();
    await act(async () => {
      window.dispatchEvent(ignored);
      await Promise.resolve();
    });

    expect(ignored.defaultPrevented).toBe(false);
    expect(mocks.undo).not.toHaveBeenCalled();
  });

  it("redoes on the shifted shortcut and ignores it when there is nothing to redo", async () => {
    usePlanStore
      .getState()
      .setSnapshot({ slots: [], sources: [], can_undo: true, can_redo: false });
    mocks.redo.mockResolvedValue({
      slots: [{ id: 3, source: 10, page: 2, rotation: 0 }],
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
    expect(usePlanStore.getState().slots).toEqual([{ id: 3, source: 10, page: 2, rotation: 0 }]);
  });

  it("logs a failed undo and leaves the plan untouched", async () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    usePlanStore.getState().setSnapshot({
      slots: [{ id: 1, source: 10, page: 0, rotation: 0 }],
      sources: [],
      can_undo: true,
      can_redo: false,
    });
    mocks.undo.mockRejectedValue(new Error("history unavailable"));
    const container = await renderToolbar();

    await click(getButton(container, "Undo"));

    expect(consoleError).toHaveBeenCalledWith("undo failed", expect.any(Error));
    expect(usePlanStore.getState().slots).toEqual([{ id: 1, source: 10, page: 0, rotation: 0 }]);
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
    // The primary button renames itself while a merge runs, so it is looked up
    // by the label the user actually sees.
    expect(getButton(container, "Merging…").disabled).toBe(true);

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
    // The completion chip carries the file name only; page totals stay in the
    // left-hand readout, where they describe the plan rather than the output.
    expect(container.textContent).not.toContain("3 pages");
    expect(mocks.unlisten).toHaveBeenCalledOnce();

    await click(getButton(container, "Show in folder"));
    expect(mocks.revealItemInDir).toHaveBeenCalledWith(outputPath);
  });

  it("titles the completion chip's file name with the full destination path", async () => {
    const outputPath = "/Users/me/Documents/a-rather-long-file-name-that-would-need-truncating.pdf";
    const pending = deferred<MergeReportDto>();
    loadOneSlot();
    mocks.save.mockResolvedValue(outputPath);
    mocks.compose.mockReturnValue(pending.promise);
    const container = await renderToolbar();

    await click(getButton(container, "Merge"));

    await act(async () => {
      pending.resolve({ page_count: 1, bytes_written: 512 });
      await pending.promise;
    });

    const named = Array.from(container.querySelectorAll("span")).find(
      (span) => span.getAttribute("title") === outputPath,
    );
    expect(named).not.toBeUndefined();
  });

  it("names the failing file in a dialog and keeps the marker after it is closed", async () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    usePlanStore.getState().setSnapshot({
      slots: [{ id: 1, source: 10, page: 0, rotation: 0 }],
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
    // The blamed names must come from the source list, not merely from the raw
    // error text that happens to spell the path out.
    const blamed = Array.from(dialog?.querySelectorAll("li") ?? []).map((item) => item.textContent);
    expect(blamed).toEqual(["report.pdf"]);
    expect(dialog?.textContent).toContain("broken xref");

    await click(getButton(container, "Close"));

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

  it("keeps every document shortcut away from the page behind the dialog", async () => {
    await openTheFailureDialog();

    act(() => {
      useUiStore.getState().selectSlots([1, 2]);
    });

    keyDown({ key: "Delete" });
    expect(mocks.removeSlots).not.toHaveBeenCalled();

    keyDown({ key: "z", metaKey: true });
    expect(mocks.undo).not.toHaveBeenCalled();

    keyDown({ key: "a", metaKey: true });
    expect(useUiStore.getState().selectedSlots.size).toBe(2);

    // The grid moves the focus and drags the selection along with it, so an
    // untouched selection is how "the arrows reached nothing" is observed.
    keyDown({ key: "ArrowRight" });
    expect(useUiStore.getState().selectedSlots.size).toBe(2);

    // Escape goes last: it is the one key the dialog itself consumes, so
    // checking it earlier would close the dialog and unblock the rest.
    keyDown({ key: "Escape" });
    expect(useUiStore.getState().selectedSlots.size).toBe(2);
  });

  it("closes the dialog on Escape and lets the shortcuts back through", async () => {
    await openTheFailureDialog();

    keyDown({ key: "Escape" });
    expect(document.querySelector('[role="alertdialog"]')).toBeNull();
    expect(useUiStore.getState().modalOpen).toBe(false);

    act(() => {
      useUiStore.getState().selectSlots([1]);
    });
    keyDown({ key: "Escape" });
    expect(useUiStore.getState().selectedSlots.size).toBe(0);
  });

  it("groups the view modes into one labelled control", async () => {
    const container = await renderToolbar();
    const group = container.querySelector('[role="group"]');

    expect(group?.getAttribute("aria-label")).toBe("View mode");
    // Both modes live inside the enclosure, which is what makes it read as a
    // single "exactly one of these" control rather than two loose buttons.
    expect(group?.querySelectorAll("button")).toHaveLength(2);
    expect(getButton(container, "Grid view").getAttribute("title")).toBe("Grid view");
  });

  it("names every centre action and spells its shortcut in the tooltip", async () => {
    const container = await renderToolbar();

    expect(getButton(container, "Undo").getAttribute("title")).toBe(
      `Undo (${shortcutHint("undo", navigator.userAgent)})`,
    );
    expect(getButton(container, "Redo").getAttribute("title")).toBe(
      `Redo (${shortcutHint("redo", navigator.userAgent)})`,
    );
    expect(getButton(container, "Rotate left").getAttribute("title")).toBe(
      `Rotate left (${shortcutHint("rotate-left", navigator.userAgent)})`,
    );
    expect(getButton(container, "Rotate right").getAttribute("title")).toBe(
      `Rotate right (${shortcutHint("rotate-right", navigator.userAgent)})`,
    );
    expect(getButton(container, "Grid view").getAttribute("aria-label")).toBe("Grid view");
    expect(getButton(container, "List view").getAttribute("aria-label")).toBe("List view");
  });

  it("locks the history buttons and renames the primary button while merging", async () => {
    const pending = deferred<MergeReportDto>();
    usePlanStore.getState().setSnapshot({
      slots: [{ id: 1, source: 10, page: 0, rotation: 0 }],
      sources: [],
      can_undo: true,
      can_redo: true,
    });
    useUiStore.getState().selectSlots([1]);
    mocks.save.mockResolvedValue("/Users/me/Documents/book.pdf");
    mocks.compose.mockReturnValue(pending.promise);
    const container = await renderToolbar();

    await click(getButton(container, "Merge"));

    expect(getButton(container, "Undo").disabled).toBe(true);
    expect(getButton(container, "Redo").disabled).toBe(true);
    expect(getButton(container, "Rotate left").disabled).toBe(true);
    expect(getButton(container, "Rotate right").disabled).toBe(true);
    expect(getButton(container, "Merging…").disabled).toBe(true);

    await act(async () => {
      pending.resolve({ page_count: 1, bytes_written: 512 });
      await pending.promise;
    });

    expect(getButton(container, "Undo").disabled).toBe(false);
    expect(getButton(container, "Rotate left").disabled).toBe(false);
    expect(getButton(container, "Rotate right").disabled).toBe(false);
    expect(getButton(container, "Merge").disabled).toBe(false);
  });
});
