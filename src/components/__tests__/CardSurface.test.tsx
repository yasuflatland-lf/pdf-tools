import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PageSlotDto } from "../../bindings/PageSlotDto";
import type { SourceFileDto } from "../../bindings/SourceFileDto";
import { usePlanStore } from "../../store/plan-store";
import { useUiStore } from "../../store/ui-store";
import { CardSurface } from "../CardSurface";
import { PageGrid } from "../PageGrid";
import { PageList } from "../PageList";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const mountedRoots: Root[] = [];
const measuredProperties = new Map<string, PropertyDescriptor | undefined>();

function loadCards(count: number): void {
  const source: SourceFileDto = {
    id: 10,
    path: "/documents/10.pdf",
    file_name: "10.pdf",
    kind: "pdf",
    grouping: "ungrouped",
    page_count: count,
    status: { kind: "ready" },
  };
  const slots: PageSlotDto[] = Array.from({ length: count }, (_, page) => ({
    id: page + 1,
    source: source.id,
    page,
    rotation: 0,
  }));

  usePlanStore.getState().setSnapshot({
    slots,
    sources: [source],
    can_undo: false,
    can_redo: false,
  });
}

async function mount(node: ReactNode): Promise<HTMLElement> {
  const container = document.createElement("div");
  document.body.append(container);
  const root = createRoot(container);
  mountedRoots.push(root);

  await act(async () => {
    root.render(node);
  });

  return container;
}

describe("CardSurface", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(new Uint8Array([1, 2, 3]));
    Object.defineProperty(URL, "createObjectURL", {
      configurable: true,
      value: vi.fn(() => "blob:thumbnail"),
    });
    Object.defineProperty(URL, "revokeObjectURL", { configurable: true, value: vi.fn() });
    for (const [name, value] of Object.entries({
      offsetWidth: 800,
      clientWidth: 800,
      offsetHeight: 600,
      clientHeight: 600,
    })) {
      measuredProperties.set(name, Object.getOwnPropertyDescriptor(HTMLElement.prototype, name));
      Object.defineProperty(HTMLElement.prototype, name, { configurable: true, value });
    }
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
    for (const [name, descriptor] of measuredProperties) {
      if (descriptor) {
        Object.defineProperty(HTMLElement.prototype, name, descriptor);
      } else {
        Reflect.deleteProperty(HTMLElement.prototype, name);
      }
    }
    measuredProperties.clear();
    vi.restoreAllMocks();
  });

  it("slices virtual rows by the configured column count", async () => {
    loadCards(4);

    const container = await mount(
      <CardSurface
        columnCount={2}
        rowHeight={332}
        viewMode="grid"
        renderCard={(card, thumbnailWidth) => (
          <article data-card={card.key} data-thumbnail-width={thumbnailWidth} />
        )}
        thumbnailWidth={360}
      />,
    );

    const surface = container.querySelector('[data-view-mode="grid"]');
    expect(surface?.getAttribute("role")).toBe("listbox");
    expect(surface?.getAttribute("aria-multiselectable")).toBe("true");
    expect(surface?.getAttribute("aria-label")).toBe("Document pages");
    expect(
      [...container.querySelectorAll<HTMLElement>("[data-card]")].map((card) => [
        card.dataset.card,
        card.dataset.thumbnailWidth,
      ]),
    ).toEqual([
      ["source-10-slot-1", "360"],
      ["source-10-slot-2", "360"],
      ["source-10-slot-3", "360"],
      ["source-10-slot-4", "360"],
    ]);
  });

  // The two views share one frame now, so the view mode is the only thing left
  // that tells them apart in the markup. Nothing else asserts it.
  it("gives each view the shared listbox frame under its own view mode", async () => {
    loadCards(3);

    for (const [view, node] of [
      ["grid", <PageGrid key="grid" />],
      ["list", <PageList key="list" />],
    ] as const) {
      const container = await mount(node);
      const surface = container.querySelector<HTMLElement>('[role="listbox"]');

      expect(surface?.dataset.viewMode).toBe(view);
      expect(surface?.getAttribute("aria-multiselectable")).toBe("true");
      expect(surface?.getAttribute("aria-label")).toBe("Document pages");
      expect(container.querySelectorAll('[role="option"]')).toHaveLength(3);
    }
  });
});
