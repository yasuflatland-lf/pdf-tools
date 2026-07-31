import {
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import { sortableKeyboardCoordinates } from "@dnd-kit/sortable";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useEffect, useMemo, useState, type RefObject } from "react";
import type { PageSlotDto } from "../bindings/PageSlotDto";
import type { SourceFileDto } from "../bindings/SourceFileDto";
import { computeDropTarget } from "../lib/drop-position";
import { groupContiguous } from "../lib/grouping";
import { nextFocusIndex, type ShortcutAction } from "../lib/keyboard";
import { rasterizeSlot, reorder } from "../lib/tauri-api";
import { createThumbnailCache, type ThumbnailCache } from "../lib/thumbnail-cache";
import { useShortcuts } from "../lib/useShortcuts";
import { usePlanStore } from "../store/plan-store";
import { useUiStore } from "../store/ui-store";

const CACHE_CAPACITY = 100;

export interface DisplayCard {
  key: string;
  slot: PageSlotDto;
  source?: SourceFileDto;
  start: number;
  pageCount: number;
  collapsed: boolean;
  collapsible: boolean;
  fileName: string;
  // Every slot the card stands for, so selecting a collapsed group selects the
  // whole run rather than only its first page.
  slotIds: number[];
}

export function usePageCards(): {
  cache: ThumbnailCache;
  cards: DisplayCard[];
  handleDragEnd: (event: DragEndEvent) => Promise<void>;
  sensors: ReturnType<typeof useSensors>;
} {
  const slots = usePlanStore((state) => state.slots);
  const sources = usePlanStore((state) => state.sources);
  const expandedSources = useUiStore((state) => state.expandedSources);
  const [cache] = useState(() =>
    createThumbnailCache({ fetcher: rasterizeSlot, capacity: CACHE_CAPACITY }),
  );
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 8 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );

  const cards = useMemo(() => {
    const sourcesById = new Map(sources.map((source) => [source.id, source]));
    let start = 0;

    return groupContiguous(slots, sources).flatMap<DisplayCard>((group) => {
      const source = sourcesById.get(group.sourceId);
      const fileName = source?.file_name ?? "Unknown source";
      const groupStart = start;
      const collapsible = source?.grouping === "grouped" && group.pageCount > 1;
      start += group.pageCount;

      if (expandedSources.has(group.sourceId)) {
        return group.slots.map((slot, slotIndex) => ({
          key: `slot-${slot.id}`,
          slot,
          source,
          start: groupStart + slotIndex,
          pageCount: 1,
          collapsed: false,
          collapsible,
          fileName,
          slotIds: [slot.id],
        }));
      }

      return [
        {
          key: group.key,
          slot: group.slots[0],
          source,
          start: groupStart,
          pageCount: group.pageCount,
          collapsed: group.pageCount > 1,
          collapsible,
          fileName,
          slotIds: group.slotIds,
        },
      ];
    });
  }, [expandedSources, slots, sources]);

  useEffect(() => () => cache.release(), [cache]);

  /**
   * The only place a drag touches the backend. While the pointer moves,
   * `rectSortingStrategy` shifts the cards with CSS transforms alone, so the
   * drop costs exactly one IPC round trip instead of one per frame. The
   * snapshot the command returns replaces the plan wholesale -- the grid never
   * keeps a second, locally reordered copy of it.
   */
  const handleDragEnd = async ({ active, over }: DragEndEvent) => {
    if (!over) {
      return;
    }

    const activeIndex = cards.findIndex((card) => card.key === active.id);
    const overIndex = cards.findIndex((card) => card.key === over.id);
    const target = computeDropTarget(cards, activeIndex, overIndex);
    if (!target) {
      return;
    }

    try {
      const snapshot = await reorder(target.from[0], target.from[1], target.to);
      usePlanStore.getState().setSnapshot(snapshot);
    } catch (error) {
      console.error("reorder failed", error);
    }
  };

  return { cache, cards, handleDragEnd, sensors };
}

interface CardFocusOptions {
  cards: DisplayCard[];
  columnCount: number;
  scrollToIndex: (rowIndex: number) => void;
}

/**
 * Arrow navigation over the cards. It lives here rather than in a view because
 * both views need it and only the column count differs -- the list is a grid one
 * column wide. The selection follows the focus so that Delete always acts on the
 * card the user is looking at.
 */
export function useCardFocus({
  cards,
  columnCount,
  scrollToIndex,
}: CardFocusOptions): number | null {
  const [focusedIndex, setFocusedIndex] = useState<number | null>(null);

  const moveFocus = (action: ShortcutAction): boolean => {
    const current = focusedIndex !== null && focusedIndex < cards.length ? focusedIndex : -1;
    const next = nextFocusIndex(action, current, cards.length, columnCount);
    if (next === null) {
      return false;
    }

    setFocusedIndex(next);
    scrollToIndex(Math.floor(next / columnCount));
    useUiStore.getState().selectSlots(cards[next].slotIds);
    return true;
  };

  useShortcuts({
    "focus-previous": () => moveFocus("focus-previous"),
    "focus-next": () => moveFocus("focus-next"),
    "focus-row-previous": () => moveFocus("focus-row-previous"),
    "focus-row-next": () => moveFocus("focus-row-next"),
  });

  return focusedIndex;
}

interface CardRowsOptions {
  cards: DisplayCard[];
  columnCount: number;
  rowHeight: number;
  scrollRef: RefObject<HTMLDivElement | null>;
}

interface CardRow {
  index: number;
  key: string | number | bigint;
  start: number;
}

export function useCardRows({ cards, columnCount, rowHeight, scrollRef }: CardRowsOptions): {
  rows: CardRow[];
  // Exposed so keyboard navigation can bring a card the virtualizer has not
  // rendered yet into view before the focus moves onto it.
  scrollToIndex: (rowIndex: number) => void;
  totalSize: number;
} {
  const rowCount = Math.ceil(cards.length / columnCount);
  const rowVirtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => rowHeight,
    // No overscan: a card is mounted only once it is on screen, and mounting is
    // what triggers its thumbnail request. Rasterizing rows the user may never
    // reach is exactly what this grid exists to avoid.
    overscan: 0,
  });
  const virtualRows = rowVirtualizer.getVirtualItems();
  // The virtualizer yields nothing while the viewport measures zero, which is
  // the case before the first layout pass and in any environment without a
  // layout engine. Showing the first row keeps the grid from looking empty.
  const renderedRows =
    virtualRows.length === 0 && rowCount > 0
      ? [{ index: 0, key: "unmeasured-row", start: 0 }]
      : virtualRows;

  return {
    rows: renderedRows,
    scrollToIndex: rowVirtualizer.scrollToIndex,
    totalSize: rowVirtualizer.getTotalSize(),
  };
}
