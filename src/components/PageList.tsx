import { useUiStore } from "../store/ui-store";
import { CardSurface, useCardSurfaceCache } from "./CardSurface";
import { PageListRow } from "./PageListRow";
import type { DisplayCard } from "./usePageCards";

const ROW_HEIGHT = 112;
const THUMBNAIL_WIDTH = 160;

export function PageList() {
  return (
    <CardSurface
      columnCount={1}
      rowHeight={ROW_HEIGHT}
      viewMode="list"
      renderCard={(card, thumbnailWidth) => (
        <PageListCard card={card} thumbnailWidth={thumbnailWidth} />
      )}
      thumbnailWidth={THUMBNAIL_WIDTH}
    />
  );
}

interface PageListCardProps {
  card: DisplayCard;
  thumbnailWidth: number;
}

function PageListCard({ card, thumbnailWidth }: PageListCardProps) {
  const cache = useCardSurfaceCache();

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
      slotId={card.slot.id}
      thumbnailWidth={thumbnailWidth}
    />
  );
}
