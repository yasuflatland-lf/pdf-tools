import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { usePlanStore } from "../../store/plan-store";
import { useUiStore } from "../../store/ui-store";
import { AppShell } from "../AppShell";

const { invoke, onDragDropEvent, removeSlots } = vi.hoisted(() => ({
  invoke: vi.fn(),
  onDragDropEvent: vi.fn(() => Promise.resolve(() => {})),
  removeSlots: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({ onDragDropEvent }),
}));
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
    removeSlots.mockReset();
    Object.defineProperty(URL, "createObjectURL", {
      configurable: true,
      value: vi.fn(() => "blob:thumbnail"),
    });
    Object.defineProperty(URL, "revokeObjectURL", { configurable: true, value: vi.fn() });
    localStorage.clear();
    useUiStore.setState({ viewMode: "grid" });
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
    useUiStore.setState({ viewMode: "grid" });
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
});
