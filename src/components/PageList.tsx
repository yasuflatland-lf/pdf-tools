import { useUiStore } from "../store/ui-store";
import { CardSurface } from "./CardSurface";
import { PageListRow } from "./PageListRow";
import { useThumbnailCache } from "./card/ThumbnailCacheProvider";
import type { DisplayCard } from "./usePageCards";

const ROW_HEIGHT = 112;
const THUMBNAIL_WIDTH = 160;

export function PageList() {
  return (
    <CardSurface
      columnCount={1}
      rowHeight={ROW_HEIGHT}
      viewMode="list"
      renderCard={(card, thumbnailWidth, selected) => (
        <PageListCard card={card} selected={selected} thumbnailWidth={thumbnailWidth} />
      )}
      thumbnailWidth={THUMBNAIL_WIDTH}
    />
  );
}

interface PageListCardProps {
  card: DisplayCard;
  selected: boolean;
  thumbnailWidth: number;
}

function PageListCard({ card, selected, thumbnailWidth }: PageListCardProps) {
  const cache = useThumbnailCache();

  return (
    <PageListRow
      cache={cache}
      collapsed={card.collapsed}
      fileName={card.fileName}
      onToggle={
        card.collapsible ? () => useUiStore.getState().toggleExpanded(card.slot.source) : undefined
      }
      pageCount={card.pageCount}
      pageNumber={card.slot.page + 1}
      rotation={card.slot.rotation}
      selected={selected}
      slotId={card.slot.id}
      thumbnailWidth={thumbnailWidth}
    />
  );
}
