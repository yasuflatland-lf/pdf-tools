import { useLayoutEffect, useRef, useState } from "react";
import { useUiStore } from "../store/ui-store";
import { CardSurface, useCardSurfaceCache } from "./CardSurface";
import { GroupCard } from "./GroupCard";
import type { DisplayCard } from "./usePageCards";

const CARD_MIN_WIDTH = 180;
const GAP = 16;
const ROW_HEIGHT = 332;
const THUMBNAIL_WIDTH = 360;

function getColumnCount(width: number): number {
  return Math.max(1, Math.floor((width + GAP) / (CARD_MIN_WIDTH + GAP)));
}

export function PageGrid() {
  const selectedSlots = useUiStore((state) => state.selectedSlots);
  const scrollRef = useRef<HTMLDivElement>(null);
  const [columnCount, setColumnCount] = useState(1);

  useLayoutEffect(() => {
    const measure = () => {
      setColumnCount(getColumnCount(scrollRef.current?.clientWidth ?? 0));
    };

    measure();
    window.addEventListener("resize", measure);
    return () => window.removeEventListener("resize", measure);
  }, []);

  return (
    <CardSurface
      columnCount={columnCount}
      rowHeight={ROW_HEIGHT}
      viewMode="grid"
      renderCard={(card, thumbnailWidth) => (
        <PageGridCard
          card={card}
          selected={card.slotIds.every((slotId) => selectedSlots.has(slotId))}
          thumbnailWidth={thumbnailWidth}
        />
      )}
      scrollRef={scrollRef}
      thumbnailWidth={THUMBNAIL_WIDTH}
    />
  );
}

interface PageGridCardProps {
  card: DisplayCard;
  selected: boolean;
  thumbnailWidth: number;
}

function PageGridCard({ card, selected, thumbnailWidth }: PageGridCardProps) {
  const cache = useCardSurfaceCache();

  return (
    <GroupCard
      cache={cache}
      collapsed={card.collapsed}
      fileName={card.fileName}
      onToggle={
        card.collapsible ? () => useUiStore.getState().toggleExpanded(card.slot.source) : undefined
      }
      pageCount={card.pageCount}
      pageNumber={card.slot.page + 1}
      slotId={card.slot.id}
      thumbnailWidth={thumbnailWidth}
      selected={selected}
    />
  );
}
