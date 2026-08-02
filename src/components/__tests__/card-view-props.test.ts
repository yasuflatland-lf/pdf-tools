import { afterEach, describe, expect, it, vi } from "vitest";
import type { ThumbnailCache } from "../../lib/thumbnail-cache";
import { usePlanStore } from "../../store/plan-store";
import { useUiStore } from "../../store/ui-store";
import type { DisplayCard } from "../usePageCards";
import { toCardViewProps } from "../card/toCardViewProps";

const cache: ThumbnailCache = {
  get: vi.fn(),
  release: vi.fn(),
};

function card(overrides: Partial<DisplayCard> = {}): DisplayCard {
  return {
    key: "source-10",
    slot: { id: 1, source: 10, page: 0, rotation: 0 },
    start: 0,
    pageCount: 1,
    collapsed: false,
    collapsible: false,
    fileName: "document.pdf",
    slotIds: [1],
    ...overrides,
  };
}

describe("toCardViewProps", () => {
  afterEach(() => {
    useUiStore.setState({ expandedSources: new Set() });
    vi.restoreAllMocks();
  });

  it("maps a zero-based slot page to a one-based page number", () => {
    const props = toCardViewProps(
      card({ slot: { id: 1, source: 10, page: 4, rotation: 0 } }),
      cache,
      360,
      false,
    );

    expect(props.pageNumber).toBe(5);
  });

  it("only provides a toggle for a foldable run", () => {
    const singlePageProps = toCardViewProps(card(), cache, 360, false);
    const foldableProps = toCardViewProps(card({ collapsible: true }), cache, 360, false);

    expect(singlePageProps.onToggle).toBeUndefined();
    expect(foldableProps.onToggle).toBeTypeOf("function");
  });

  it("rotates every slot represented by the card", async () => {
    const rotate = vi.spyOn(usePlanStore.getState(), "rotate").mockResolvedValue();
    const props = toCardViewProps(card({ slotIds: [11, 12, 13] }), cache, 360, false);

    props.onRotate?.(1);
    await vi.waitFor(() => expect(rotate).toHaveBeenCalledWith([11, 12, 13], 1));
  });
});
