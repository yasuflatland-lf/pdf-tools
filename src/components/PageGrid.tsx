import { useLayoutEffect, useRef, useState } from "react";
import { CardSurface } from "./CardSurface";
import { GroupCard } from "./GroupCard";
import { useThumbnailCache } from "./card/ThumbnailCacheProvider";
import { toCardViewProps } from "./card/toCardViewProps";

const CARD_MIN_WIDTH = 180;
const GAP = 16;
const ROW_HEIGHT = 332;
const THUMBNAIL_WIDTH = 360;

function getColumnCount(width: number): number {
  return Math.max(1, Math.floor((width + GAP) / (CARD_MIN_WIDTH + GAP)));
}

export function PageGrid() {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [columnCount, setColumnCount] = useState(1);
  const cache = useThumbnailCache();

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
      renderCard={(card, thumbnailWidth, selected) => (
        <GroupCard {...toCardViewProps(card, cache, thumbnailWidth, selected)} />
      )}
      scrollRef={scrollRef}
      thumbnailWidth={THUMBNAIL_WIDTH}
    />
  );
}
