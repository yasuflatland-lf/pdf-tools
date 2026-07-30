import { useVirtualizer } from "@tanstack/react-virtual";
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { PageSlotDto } from "../bindings/PageSlotDto";
import type { SourceFileDto } from "../bindings/SourceFileDto";
import { groupContiguous } from "../lib/grouping";
import { rasterizeSlot } from "../lib/tauri-api";
import { createThumbnailCache } from "../lib/thumbnail-cache";
import { usePlanStore } from "../store/plan-store";
import { useUiStore } from "../store/ui-store";
import { PageCard } from "./PageCard";

const CARD_MIN_WIDTH = 180;
const GAP = 16;
const ROW_HEIGHT = 332;
const THUMBNAIL_WIDTH = 360;
const CACHE_CAPACITY = 100;

interface DisplayCard {
  key: string;
  slot: PageSlotDto;
  source?: SourceFileDto;
  pageCount: number;
  collapsed: boolean;
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

  const cards = useMemo(() => {
    const sourcesById = new Map(sources.map((source) => [source.id, source]));

    return groupContiguous(slots, sources).flatMap<DisplayCard>((group) => {
      const source = sourcesById.get(group.sourceId);
      if (expandedSources.has(group.sourceId)) {
        return group.slots.map((slot) => ({
          key: `slot-${slot.id}`,
          slot,
          source,
          pageCount: 1,
          collapsed: false,
        }));
      }

      return [
        {
          key: group.key,
          slot: group.slots[0],
          source,
          pageCount: group.pageCount,
          collapsed: group.pageCount > 1,
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

  return (
    <div ref={scrollRef} className="h-full overflow-y-auto" aria-label="Document pages">
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
              {rowCards.map((card) => (
                <PageCard
                  key={card.key}
                  cache={cache}
                  collapsed={card.collapsed}
                  fileName={card.source?.file_name ?? "Unknown source"}
                  pageCount={card.pageCount}
                  pageNumber={card.slot.page + 1}
                  slotId={card.slot.id}
                  thumbnailWidth={THUMBNAIL_WIDTH}
                />
              ))}
            </div>
          );
        })}
      </div>
    </div>
  );
}
