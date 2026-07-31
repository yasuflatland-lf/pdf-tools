import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PageSlotDto } from "../../bindings/PageSlotDto";
import type { PlanSnapshot } from "../../bindings/PlanSnapshot";
import type { SourceFileDto } from "../../bindings/SourceFileDto";
import { usePlanStore } from "../../store/plan-store";
import { useUiStore } from "../../store/ui-store";
import { PageGrid } from "../PageGrid";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const mountedRoots: Root[] = [];

function source(id: number, grouping: SourceFileDto["grouping"], pageCount: number): SourceFileDto {
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

async function renderGrid(): Promise<HTMLElement> {
  const container = document.createElement("div");
  document.body.append(container);
  const root = createRoot(container);
  mountedRoots.push(root);

  await act(async () => {
    root.render(<PageGrid />);
  });

  return container;
}

function requestedSlotIds(): number[] {
  return invoke.mock.calls.map((call) => (call[1] as { slotId: number }).slotId);
}

/**
 * jsdom has no layout engine, so every element measures zero and the virtualizer
 * would render nothing. Giving it a viewport lets the test exercise the real
 * windowing maths: 800px wide fits four columns, 600px tall fits two rows.
 */
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

/**
 * dnd-kit resolves a drop from the droppables' rects, and jsdom measures every
 * element as zero-sized. Laying the sortable cards out as a row of 200px cells
 * gives the keyboard sensor a geometry to move through.
 */
function layOutSortableCards(container: HTMLElement): HTMLElement[] {
  const cards = [...container.querySelectorAll<HTMLElement>('[aria-roledescription="sortable"]')];

  for (const [index, card] of cards.entries()) {
    const left = index * 200;
    const rect = {
      x: left,
      y: 0,
      left,
      top: 0,
      right: left + 180,
      bottom: 300,
      width: 180,
      height: 300,
    };
    Object.defineProperty(card, "getBoundingClientRect", {
      configurable: true,
      value: () => ({ ...rect, toJSON: () => rect }),
    });
  }

  return cards;
}

async function pressKey(target: HTMLElement, code: string): Promise<void> {
  await act(async () => {
    target.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, cancelable: true, code }));
    // dnd-kit measures and reconciles across animation frames, which jsdom only
    // runs on a timer.
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

describe("PageGrid", () => {
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
    useUiStore.setState({ expandedSources: new Set(), selectedSlots: new Set() });
    stopMeasuringElements();
    vi.restoreAllMocks();
  });

  it("rasterizes only the cards on screen, not the whole document", async () => {
    load(source(10, "ungrouped", 200), 200);

    await renderGrid();

    const requested = requestedSlotIds();
    expect(requested).toContain(1);
    expect(requested).not.toContain(200);
    // Four columns over the two rows that fit in the 600px viewport.
    expect(requested).toHaveLength(8);
  });

  // A scroll container's clientWidth excludes its vertical scrollbar; every
  // element outside it counts the space that scrollbar occupies. macOS overlay
  // scrollbars hide the difference, Windows WebView2 does not, and 15px is
  // enough to flip `getColumnCount` at a track boundary -- 1145px is five
  // columns, 1160px is six, and six columns leave every card under
  // CARD_MIN_WIDTH. Hence both widths: only measuring the listbox itself gives
  // five.
  it("counts columns from the scrolling element, whose width excludes the scrollbar", async () => {
    load(source(10, "ungrouped", 12), 12);

    const container = await renderGrid();
    const listbox = container.querySelector<HTMLElement>('[role="listbox"]');
    expect(listbox).not.toBeNull();
    Object.defineProperty(listbox, "clientWidth", { configurable: true, value: 1145 });

    for (let ancestor = listbox?.parentElement; ancestor; ancestor = ancestor.parentElement) {
      Object.defineProperty(ancestor, "clientWidth", { configurable: true, value: 1160 });
      if (ancestor === container) break;
    }

    await act(async () => {
      window.dispatchEvent(new Event("resize"));
    });

    const row = container.querySelector<HTMLElement>('[style*="grid-template-columns"]');
    expect(row?.style.gridTemplateColumns).toBe("repeat(5, minmax(0, 1fr))");
  });

  it("collapses a grouped source into one card that reports its page count", async () => {
    load(source(10, "grouped", 3), 3);

    const container = await renderGrid();

    expect(container.querySelectorAll("article")).toHaveLength(1);
    expect(container.textContent).toContain("10.pdf");
    expect(container.textContent).toContain("3 pages");
    expect(requestedSlotIds()).toEqual([1]);
  });

  it("expands a source into per-page cards when the UI store says so", async () => {
    load(source(10, "grouped", 3), 3);
    useUiStore.setState({ expandedSources: new Set([10]) });

    const container = await renderGrid();

    expect(container.textContent).toContain("Page 1");
    expect(container.textContent).not.toContain("3 pages");
  });

  it("expands a collapsed grouped card when its control is clicked", async () => {
    load(source(10, "grouped", 3), 3);
    const container = await renderGrid();
    const expand = container.querySelector<HTMLButtonElement>('[aria-label="Expand 10.pdf"]');

    expect(expand).not.toBeNull();
    await act(async () => {
      expand?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(useUiStore.getState().expandedSources.has(10)).toBe(true);
    expect(container.querySelectorAll("article")).toHaveLength(3);
    expect(container.textContent).toContain("Page 1");
    expect(container.textContent).not.toContain("3 pages");
  });

  it("folds an expanded source back into one card from any of its pages", async () => {
    load(source(10, "grouped", 3), 3);
    useUiStore.setState({ expandedSources: new Set([10]) });
    const container = await renderGrid();
    const collapse = container.querySelectorAll<HTMLButtonElement>(
      '[aria-label="Collapse 10.pdf"]',
    );

    expect(collapse).toHaveLength(3);
    await act(async () => {
      collapse[2].dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(useUiStore.getState().expandedSources.has(10)).toBe(false);
    expect(container.querySelectorAll("article")).toHaveLength(1);
    expect(container.textContent).toContain("3 pages");
  });

  it("keeps a keypress on the expand control away from the drag sensor", async () => {
    load(source(10, "grouped", 3), 3);
    const container = await renderGrid();
    const expand = container.querySelector<HTMLElement>('[aria-label="Expand 10.pdf"]');
    const sortable = expand?.closest<HTMLElement>('[aria-roledescription="sortable"]');
    layOutSortableCards(container);

    // The faded card is `isDragging`: the same key on the card itself does pick
    // it up, so the full opacity below really is the control swallowing the key.
    await pressKey(sortable as HTMLElement, "Space");
    expect(sortable?.style.opacity).toBe("0.4");
    await pressKey(sortable as HTMLElement, "Escape");

    await pressKey(expand as HTMLElement, "Space");

    // dnd-kit's keyboard sensor listens on the sortable wrapper; letting the
    // key through would pick the card up instead of toggling the group.
    expect(sortable?.style.opacity).toBe("1");
  });

  it("renders no expand or collapse control for an ungrouped source", async () => {
    load(source(10, "ungrouped", 2), 2);

    const container = await renderGrid();

    expect(container.querySelector('[aria-label^="Expand "]')).toBeNull();
    expect(container.querySelector('[aria-label^="Collapse "]')).toBeNull();
  });

  it("renders no expand control for a grouped source with a single page", async () => {
    load(source(10, "grouped", 1), 1);

    const container = await renderGrid();

    expect(container.querySelector('[aria-label^="Expand "]')).toBeNull();
    expect(container.querySelector('[aria-label^="Collapse "]')).toBeNull();
  });

  it("moves focus and selection to a card with the arrow keys", async () => {
    load(source(10, "ungrouped", 3), 3);
    await renderGrid();

    await act(async () => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", {
          bubbles: true,
          cancelable: true,
          key: "ArrowRight",
        }),
      );
    });

    expect(document.activeElement?.getAttribute("role")).toBe("option");
    expect(document.activeElement?.getAttribute("aria-selected")).toBe("true");
    expect(useUiStore.getState().selectedSlots).toEqual(new Set([1]));
  });

  it("keeps focus on the same slot when an earlier card is deleted", async () => {
    const sourceFile = source(10, "ungrouped", 5);
    const originalSlots = slots(sourceFile.id, 5);
    load(sourceFile, 5);
    await renderGrid();

    for (let index = 0; index < 4; index += 1) {
      await pressArrow("ArrowRight");
    }
    expect(useUiStore.getState().selectedSlots).toEqual(new Set([4]));

    await act(async () => {
      usePlanStore.getState().setSnapshot({
        slots: originalSlots.slice(1),
        sources: [sourceFile],
        can_undo: true,
        can_redo: false,
      });
    });

    expect(document.activeElement?.textContent).toContain("Page 4");
    expect(useUiStore.getState().selectedSlots).toEqual(new Set([4]));
  });

  it("selects the replacement card when the focused card is deleted", async () => {
    const sourceFile = source(10, "ungrouped", 5);
    const originalSlots = slots(sourceFile.id, 5);
    load(sourceFile, 5);
    await renderGrid();

    for (let index = 0; index < 3; index += 1) {
      await pressArrow("ArrowRight");
    }

    await act(async () => {
      usePlanStore.getState().setSnapshot({
        slots: originalSlots.filter((slot) => slot.id !== 3),
        sources: [sourceFile],
        can_undo: true,
        can_redo: false,
      });
    });

    expect(document.activeElement?.textContent).toContain("Page 4");
    expect(useUiStore.getState().selectedSlots).toEqual(new Set([4]));
  });

  it("keeps focus and selection on the replacement card after undo", async () => {
    const sourceFile = source(10, "ungrouped", 5);
    const originalSlots = slots(sourceFile.id, 5);
    load(sourceFile, 5);
    await renderGrid();

    for (let index = 0; index < 3; index += 1) {
      await pressArrow("ArrowRight");
    }

    await act(async () => {
      usePlanStore.getState().setSnapshot({
        slots: originalSlots.filter((slot) => slot.id !== 3),
        sources: [sourceFile],
        can_undo: true,
        can_redo: false,
      });
    });

    await act(async () => {
      usePlanStore.getState().setSnapshot({
        slots: originalSlots,
        sources: [sourceFile],
        can_undo: false,
        can_redo: true,
      });
    });

    expect(document.activeElement?.textContent).toContain("Page 4");
    expect(useUiStore.getState().selectedSlots).toEqual(new Set([4]));
  });

  it("keeps focus and selection on the replacement card after an unrelated reorder", async () => {
    const sourceFile = source(10, "ungrouped", 5);
    const originalSlots = slots(sourceFile.id, 5);
    const slotsAfterDelete = originalSlots.filter((slot) => slot.id !== 3);
    load(sourceFile, 5);
    await renderGrid();

    for (let index = 0; index < 3; index += 1) {
      await pressArrow("ArrowRight");
    }

    await act(async () => {
      usePlanStore.getState().setSnapshot({
        slots: slotsAfterDelete,
        sources: [sourceFile],
        can_undo: true,
        can_redo: false,
      });
    });

    await act(async () => {
      usePlanStore.getState().setSnapshot({
        slots: [...slotsAfterDelete.slice(1), slotsAfterDelete[0]],
        sources: [sourceFile],
        can_undo: true,
        can_redo: false,
      });
    });

    expect(document.activeElement?.textContent).toContain("Page 4");
    expect(useUiStore.getState().selectedSlots).toEqual(new Set([4]));
  });

  it("follows a focused card when it is reordered to the end", async () => {
    const sourceFile = source(10, "ungrouped", 5);
    const originalSlots = slots(sourceFile.id, 5);
    load(sourceFile, 5);
    const container = await renderGrid();

    for (let index = 0; index < 3; index += 1) {
      await pressArrow("ArrowRight");
    }

    await act(async () => {
      usePlanStore.getState().setSnapshot({
        slots: [...originalSlots.slice(0, 2), ...originalSlots.slice(3), originalSlots[2]],
        sources: [sourceFile],
        can_undo: true,
        can_redo: false,
      });
    });

    const options = container.querySelectorAll('[role="option"]');
    expect(options[4]).toBe(document.activeElement);
    expect(options[4]?.textContent).toContain("Page 3");
    expect(useUiStore.getState().selectedSlots).toEqual(new Set([3]));
  });

  // `aria-selected` is only announced on an option the listbox owns. Any role
  // between the two -- dnd-kit puts `role="button"` on its drag wrapper unless
  // told otherwise -- silently drops the selection state for a screen reader.
  it("lets the listbox own the option cards with no role in between", async () => {
    load(source(10, "ungrouped", 3), 3);

    const container = await renderGrid();

    const option = container.querySelector('[role="option"]');
    expect(option).not.toBeNull();
    expect(option?.parentElement?.closest("[role]")?.getAttribute("role")).toBe("listbox");
  });

  it("shows a thumbnail once the rasterized bytes arrive", async () => {
    load(source(10, "grouped", 1), 1);

    const container = await renderGrid();

    expect(invoke).toHaveBeenCalledWith("rasterize_slot", expect.objectContaining({ slotId: 1 }));
    expect(container.querySelector("img")?.getAttribute("src")).toBe("blob:thumbnail");
  });

  it("sends exactly one reorder command when a card is dropped on another", async () => {
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

    const container = await renderGrid();
    const cards = layOutSortableCards(container);
    // A keyboard drag: pick the first card up, step right onto the second, drop.
    await pressKey(cards[0], "Space");
    await pressKey(cards[0], "ArrowRight");
    await pressKey(cards[0], "Space");

    expect(invoke.mock.calls.filter((call) => call[0] === "reorder")).toEqual([
      ["reorder", { fromStart: 0, fromEnd: 1, to: 1 }],
    ]);
    expect(usePlanStore.getState().slots.map((slot) => slot.id)).toEqual([2, 1, 3]);
  });

  it("falls back to a placeholder when rasterization fails", async () => {
    invoke.mockRejectedValue("slot 1 was not found");
    load(source(10, "grouped", 1), 1);

    const container = await renderGrid();

    expect(container.querySelector("img")).toBeNull();
    expect(container.querySelector('[aria-label="Thumbnail unavailable"]')).not.toBeNull();
  });

  it("shows a visible badge when a thumbnail cannot be rendered", async () => {
    invoke.mockRejectedValue(new Error("boom"));
    load(source(10, "grouped", 1), 1);

    const container = await renderGrid();

    expect(container.textContent).toContain("Thumbnail unavailable");
  });
});
