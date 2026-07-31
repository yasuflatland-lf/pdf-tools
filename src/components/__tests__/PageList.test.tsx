import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PageSlotDto } from "../../bindings/PageSlotDto";
import type { PlanSnapshot } from "../../bindings/PlanSnapshot";
import type { SourceFileDto } from "../../bindings/SourceFileDto";
import { usePlanStore } from "../../store/plan-store";
import { useUiStore } from "../../store/ui-store";
import { PageList } from "../PageList";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const mountedRoots: Root[] = [];

function source(id: number, grouping: string, pageCount: number): SourceFileDto {
  return {
    id,
    path: `/documents/${id}.pdf`,
    file_name: `${id}.pdf`,
    kind: "pdf",
    grouping,
    page_count: pageCount,
    status: { kind: "ready" },
  };
}

function slots(sourceId: number, pageCount: number): PageSlotDto[] {
  return Array.from({ length: pageCount }, (_, page) => ({
    id: page + 1,
    source: sourceId,
    page,
  }));
}

function load(sourceFile: SourceFileDto, pageCount: number): void {
  usePlanStore.getState().setSnapshot({
    slots: slots(sourceFile.id, pageCount),
    sources: [sourceFile],
    can_undo: false,
    can_redo: false,
  });
}

async function renderList(): Promise<HTMLElement> {
  const container = document.createElement("div");
  document.body.append(container);
  const root = createRoot(container);
  mountedRoots.push(root);

  await act(async () => {
    root.render(<PageList />);
  });

  return container;
}

const VIEWPORT = { offsetWidth: 800, clientWidth: 800, offsetHeight: 600, clientHeight: 600 };
const measuredProperties = new Map<string, PropertyDescriptor | undefined>();

function giveEveryElementAViewport(): void {
  for (const [name, value] of Object.entries(VIEWPORT)) {
    if (!measuredProperties.has(name)) {
      measuredProperties.set(name, Object.getOwnPropertyDescriptor(HTMLElement.prototype, name));
    }
    Object.defineProperty(HTMLElement.prototype, name, { configurable: true, value });
  }
}

function layOutSortableRows(container: HTMLElement): HTMLElement[] {
  const rows = [...container.querySelectorAll<HTMLElement>('[aria-roledescription="sortable"]')];

  for (const [index, row] of rows.entries()) {
    const top = index * 100;
    const rect = {
      x: 0,
      y: top,
      left: 0,
      top,
      right: 800,
      bottom: top + 84,
      width: 800,
      height: 84,
    };
    Object.defineProperty(row, "getBoundingClientRect", {
      configurable: true,
      value: () => ({ ...rect, toJSON: () => rect }),
    });
  }

  return rows;
}

async function pressKey(target: HTMLElement, code: string): Promise<void> {
  await act(async () => {
    target.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, cancelable: true, code }));
    await new Promise((resolve) => setTimeout(resolve, 32));
  });
}

async function pressArrow(key: string): Promise<void> {
  await act(async () => {
    window.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, cancelable: true, key }));
  });
}

function stopMeasuringElements(): void {
  for (const [name, descriptor] of measuredProperties) {
    if (descriptor) {
      Object.defineProperty(HTMLElement.prototype, name, descriptor);
    } else {
      Reflect.deleteProperty(HTMLElement.prototype, name);
    }
  }
}

describe("PageList", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(new Uint8Array([1, 2, 3]));
    Object.defineProperty(URL, "createObjectURL", {
      configurable: true,
      value: vi.fn(() => "blob:thumbnail"),
    });
    Object.defineProperty(URL, "revokeObjectURL", { configurable: true, value: vi.fn() });
    giveEveryElementAViewport();
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
    useUiStore.setState({
      expandedSources: new Set(),
      modalOpen: false,
      selectedSlots: new Set(),
    });
    stopMeasuringElements();
    vi.restoreAllMocks();
  });

  it("renders one row per display group", async () => {
    load(source(10, "ungrouped", 3), 3);

    const container = await renderList();

    expect(container.querySelectorAll("article")).toHaveLength(3);
  });

  it("collapses a grouped source into one row that reports its page count", async () => {
    load(source(10, "grouped", 3), 3);

    const container = await renderList();

    expect(container.querySelectorAll("article")).toHaveLength(1);
    expect(container.textContent).toContain("10.pdf");
    expect(container.textContent).toContain("3 pages");
  });

  it("expands a source into per-page rows when the UI store says so", async () => {
    load(source(10, "grouped", 3), 3);
    useUiStore.setState({ expandedSources: new Set([10]) });

    const container = await renderList();

    expect(container.querySelectorAll("article")).toHaveLength(3);
    expect(container.textContent).toContain("Page 1");
    expect(container.textContent).not.toContain("3 pages");
  });

  it("expands a collapsed group when its control is clicked", async () => {
    load(source(10, "grouped", 3), 3);
    const container = await renderList();
    const expand = container.querySelector<HTMLButtonElement>('[aria-label="Expand 10.pdf"]');

    expect(expand).not.toBeNull();
    await act(async () => {
      expand?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(useUiStore.getState().expandedSources.has(10)).toBe(true);
    expect(container.querySelectorAll("article")).toHaveLength(3);
  });

  it("sends one reorder command with the grid coordinates after a keyboard drag", async () => {
    const reordered: PlanSnapshot = {
      slots: [
        { id: 2, source: 10, page: 1 },
        { id: 1, source: 10, page: 0 },
        { id: 3, source: 10, page: 2 },
      ],
      sources: [source(10, "ungrouped", 3)],
      can_undo: true,
      can_redo: false,
    };
    invoke.mockImplementation((command: string) =>
      Promise.resolve(command === "reorder" ? reordered : new Uint8Array([1, 2, 3])),
    );
    load(source(10, "ungrouped", 3), 3);

    const container = await renderList();
    const rows = layOutSortableRows(container);
    await pressKey(rows[0], "Space");
    await pressKey(rows[0], "ArrowDown");
    await pressKey(rows[0], "Space");

    expect(invoke.mock.calls.filter((call) => call[0] === "reorder")).toEqual([
      ["reorder", { fromStart: 0, fromEnd: 1, to: 1 }],
    ]);
    expect(usePlanStore.getState().slots.map((slot) => slot.id)).toEqual([2, 1, 3]);
  });

  it("shows a visible badge when a thumbnail cannot be rendered", async () => {
    invoke.mockRejectedValue(new Error("boom"));
    load(source(10, "grouped", 1), 1);

    const container = await renderList();

    expect(container.textContent).toContain("サムネイルを表示できません");
  });

  it("does not label every row with a status that cannot be anything but ready", async () => {
    load(source(10, "grouped", 1), 1);

    const container = await renderList();

    expect(container.textContent).not.toContain("Ready");
  });

  it("moves focus and selection with the arrow keys", async () => {
    load(source(10, "ungrouped", 3), 3);
    const container = await renderList();
    const options = [...container.querySelectorAll<HTMLElement>('[role="option"]')];

    expect(options).toHaveLength(3);
    await pressArrow("ArrowDown");
    expect(options[0].getAttribute("aria-selected")).toBe("true");
    expect(options[0]).toBe(document.activeElement);

    await pressArrow("ArrowDown");
    expect(options[1].getAttribute("aria-selected")).toBe("true");
  });

  it("treats left and right as previous and next in a single column", async () => {
    load(source(10, "ungrouped", 3), 3);
    const container = await renderList();

    await pressArrow("ArrowRight");
    await pressArrow("ArrowRight");

    expect(container.querySelectorAll('[role="option"]')[1]?.getAttribute("aria-selected")).toBe(
      "true",
    );
  });

  it("does not move focus or selection while a modal is open", async () => {
    load(source(10, "ungrouped", 3), 3);
    useUiStore.setState({ modalOpen: true });
    const container = await renderList();
    const options = [...container.querySelectorAll<HTMLElement>('[role="option"]')];

    await pressArrow("ArrowRight");

    expect(options.some((option) => option.getAttribute("aria-selected") === "true")).toBe(false);
    expect(options.includes(document.activeElement as HTMLElement)).toBe(false);
  });

  it("lets the listbox own the option rows with no role in between", async () => {
    load(source(10, "ungrouped", 3), 3);
    const container = await renderList();
    const listbox = container.querySelector('[role="listbox"]');

    expect(listbox).not.toBeNull();
    expect(listbox?.getAttribute("aria-multiselectable")).toBe("true");
    for (const option of container.querySelectorAll('[role="option"]')) {
      expect(option.closest('[role="listbox"]')).toBe(listbox);
    }
  });
});
