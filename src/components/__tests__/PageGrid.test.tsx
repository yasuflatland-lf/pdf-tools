import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PageSlotDto } from "../../bindings/PageSlotDto";
import type { SourceFileDto } from "../../bindings/SourceFileDto";
import { usePlanStore } from "../../store/plan-store";
import { useUiStore } from "../../store/ui-store";
import { PageGrid } from "../PageGrid";

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
    useUiStore.setState({ expandedSources: new Set() });
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

  it("shows a thumbnail once the rasterized bytes arrive", async () => {
    load(source(10, "grouped", 1), 1);

    const container = await renderGrid();

    expect(invoke).toHaveBeenCalledWith("rasterize_slot", expect.objectContaining({ slotId: 1 }));
    expect(container.querySelector("img")?.getAttribute("src")).toBe("blob:thumbnail");
  });

  it("falls back to a placeholder when rasterization fails", async () => {
    invoke.mockRejectedValue("slot 1 was not found");
    load(source(10, "grouped", 1), 1);

    const container = await renderGrid();

    expect(container.querySelector("img")).toBeNull();
    expect(container.querySelector('[aria-label="Thumbnail unavailable"]')).not.toBeNull();
  });
});
