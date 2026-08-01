import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { usePlanStore } from "../../store/plan-store";
import { useUiStore } from "../../store/ui-store";
import { AppShell } from "../AppShell";

const { invoke, onDragDropEvent, open, removeSlots } = vi.hoisted(() => ({
  invoke: vi.fn(),
  onDragDropEvent: vi.fn(() => Promise.resolve(() => {})),
  open: vi.fn(),
  removeSlots: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({ onDragDropEvent }),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ ask: vi.fn(), open }));
vi.mock("../../lib/tauri-api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../lib/tauri-api")>()),
  removeSlots,
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const mountedRoots: Root[] = [];

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

async function click(button: HTMLButtonElement): Promise<void> {
  await act(async () => {
    button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });
}

function deleteShortcut(): KeyboardEvent {
  return new KeyboardEvent("keydown", {
    bubbles: true,
    cancelable: true,
    key: "Delete",
  });
}

function loadPages(): void {
  usePlanStore.getState().setSnapshot({
    slots: [
      { id: 1, source: 10, page: 0, rotation: 0 },
      { id: 2, source: 10, page: 1, rotation: 0 },
    ],
    sources: [
      {
        id: 10,
        path: "/documents/report.pdf",
        file_name: "report.pdf",
        kind: "pdf",
        grouping: "ungrouped",
        page_count: 2,
        status: { kind: "ready" },
      },
    ],
    can_undo: false,
    can_redo: false,
  });
}

describe("AppShell", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(new Uint8Array([1, 2, 3]));
    open.mockReset();
    // A cancelled picker keeps a click from running the real add procedure.
    open.mockResolvedValue(null);
    removeSlots.mockReset();
    Object.defineProperty(URL, "createObjectURL", {
      configurable: true,
      value: vi.fn(() => "blob:thumbnail"),
    });
    Object.defineProperty(URL, "revokeObjectURL", { configurable: true, value: vi.fn() });
    localStorage.clear();
    useUiStore.setState({ viewMode: "grid", sourceNotice: null, isIngesting: false });
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
    useUiStore.setState({ viewMode: "grid", sourceNotice: null, isIngesting: false });
    vi.restoreAllMocks();
  });

  it("swaps the grid for the list when the toolbar switches the view", async () => {
    loadPages();
    const container = await renderShell();

    expect(container.querySelector('[data-view-mode="grid"]')).not.toBeNull();
    expect(container.querySelector('[data-view-mode="list"]')).toBeNull();

    await click(getButton(container, "List view"));

    expect(container.querySelector('[data-view-mode="list"]')).not.toBeNull();
    expect(container.querySelector('[data-view-mode="grid"]')).toBeNull();
    expect(container.textContent).toContain("report.pdf");

    await click(getButton(container, "Grid view"));

    expect(container.querySelector('[data-view-mode="grid"]')).not.toBeNull();
  });

  it("keeps the empty-state prompt in either view", async () => {
    useUiStore.setState({ viewMode: "list" });

    const container = await renderShell();

    expect(container.textContent).toContain("Drop PDFs or images here");
    expect(container.querySelector('[data-view-mode="list"]')).toBeNull();
  });

  it("offers both pickers in the empty state", async () => {
    const container = await renderShell();

    expect(getButton(container, "Choose files…").disabled).toBe(false);
    expect(getButton(container, "Choose folder…").disabled).toBe(false);
  });

  it("sends each picker to its own dialog", async () => {
    const container = await renderShell();

    await click(getButton(container, "Choose files…"));

    expect(open).toHaveBeenLastCalledWith({
      multiple: true,
      filters: [{ name: "PDFs and images", extensions: ["pdf", "jpg", "jpeg", "png", "gif"] }],
    });

    await click(getButton(container, "Choose folder…"));

    expect(open).toHaveBeenLastCalledWith({ directory: true });
    expect(open).toHaveBeenCalledTimes(2);
  });

  it("disables both pickers while an add is in flight", async () => {
    const container = await renderShell();

    await act(async () => {
      useUiStore.getState().setIngesting(true);
    });

    expect(getButton(container, "Choose files…").disabled).toBe(true);
    expect(getButton(container, "Choose folder…").disabled).toBe(true);
  });

  it("leaves Delete alone when the selection is empty", async () => {
    useUiStore.getState().clearSelection();
    await renderShell();
    const event = deleteShortcut();

    await act(async () => {
      window.dispatchEvent(event);
      await Promise.resolve();
    });

    expect(event.defaultPrevented).toBe(false);
    expect(removeSlots).not.toHaveBeenCalled();
  });

  it("shows the source notice above the document and dismisses it", async () => {
    useUiStore.getState().setSourceNotice('No PDFs or images found in "Scans".');
    const container = await renderShell();

    expect(container.textContent).toContain('No PDFs or images found in "Scans".');

    await click(getButton(container, "Dismiss"));

    expect(useUiStore.getState().sourceNotice).toBeNull();
    expect(container.textContent).not.toContain('No PDFs or images found in "Scans".');
  });

  it("keeps the notice visible once the document has pages", async () => {
    loadPages();
    useUiStore.getState().setSourceNotice("No PDFs or images found in the selected folders.");
    const container = await renderShell();

    // The empty state is gone here, so a notice rendered inside it would have
    // nowhere to appear -- which is why it lives above the document instead.
    expect(container.textContent).not.toContain("Drop PDFs or images here");
    expect(container.textContent).toContain("No PDFs or images found in the selected folders.");
  });
});
