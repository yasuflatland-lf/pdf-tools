import {
  DndContext,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import {
  rectSortingStrategy,
  SortableContext,
  sortableKeyboardCoordinates,
} from "@dnd-kit/sortable";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { PageSlotDto } from "../bindings/PageSlotDto";
import type { SourceFileDto } from "../bindings/SourceFileDto";
import { computeDropTarget } from "../lib/drop-position";
import { groupContiguous } from "../lib/grouping";
import { rasterizeSlot, reorder } from "../lib/tauri-api";
import { createThumbnailCache } from "../lib/thumbnail-cache";
import { usePlanStore } from "../store/plan-store";
import { useUiStore } from "../store/ui-store";
import { GroupCard } from "./GroupCard";
import { SortableCard } from "./SortableCard";

const CARD_MIN_WIDTH = 180;
const GAP = 16;
const ROW_HEIGHT = 332;
const THUMBNAIL_WIDTH = 360;
const CACHE_CAPACITY = 100;

interface DisplayCard {
  key: string;
  slot: PageSlotDto;
  source?: SourceFileDto;
  start: number;
  pageCount: number;
  collapsed: boolean;
  collapsible: boolean;
}

function getColumnCount(width: number): number {
  return Math.max(1, Math.floor((width + GAP) / (CARD_MIN_WIDTH + GAP)));
}

export function PageGrid() {
  const slots = usePlanStore((state) => state.slots);
  const sources = usePlanStore((state) => state.sources);
  const expandedSources = useUiStore((state) => state.expandedSources);
  const scrollRef = useRef<HTMLDivElement>(null);
  const [columnCount, setColumnCount] = useState(1);
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
        },
      ];
    });
  }, [expandedSources, slots, sources]);

  const rowCount = Math.ceil(cards.length / columnCount);
  const rowVirtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT,
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

  useLayoutEffect(() => {
    const measure = () => {
      setColumnCount(getColumnCount(scrollRef.current?.clientWidth ?? 0));
    };

    measure();
    window.addEventListener("resize", measure);
    return () => window.removeEventListener("resize", measure);
  }, []);

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

  return (
    <DndContext sensors={sensors} onDragEnd={handleDragEnd}>
      <div ref={scrollRef} className="h-full overflow-y-auto" aria-label="Document pages">
        <SortableContext items={cards.map((card) => card.key)} strategy={rectSortingStrategy}>
          <div className="relative w-full" style={{ height: `${rowVirtualizer.getTotalSize()}px` }}>
            {renderedRows.map((virtualRow) => {
              const rowCards = cards.slice(
                virtualRow.index * columnCount,
                (virtualRow.index + 1) * columnCount,
              );

              return (
                <div
                  key={virtualRow.key}
                  className="absolute top-0 left-0 grid w-full gap-4 pb-4"
                  style={{
                    gridTemplateColumns: `repeat(${columnCount}, minmax(0, 1fr))`,
                    transform: `translateY(${virtualRow.start}px)`,
                  }}
                >
                  {rowCards.map((card) => {
                    const fileName = card.source?.file_name ?? "Unknown source";

                    return (
                      <SortableCard key={card.key} id={card.key} label={fileName}>
                        <GroupCard
                          cache={cache}
                          collapsed={card.collapsed}
                          fileName={fileName}
                          onToggle={
                            card.collapsible
                              ? () => useUiStore.getState().toggleExpanded(card.slot.source)
                              : undefined
                          }
                          pageCount={card.pageCount}
                          pageNumber={card.slot.page + 1}
                          slotId={card.slot.id}
                          thumbnailWidth={THUMBNAIL_WIDTH}
                        />
                      </SortableCard>
                    );
                  })}
                </div>
              );
            })}
          </div>
        </SortableContext>
      </div>
    </DndContext>
  );
}
