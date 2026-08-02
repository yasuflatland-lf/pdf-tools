import { CardSurface } from "./CardSurface";
import { PageListRow } from "./PageListRow";
import { useThumbnailCache } from "./card/ThumbnailCacheProvider";
import { toCardViewProps } from "./card/toCardViewProps";

const ROW_HEIGHT = 112;
const THUMBNAIL_WIDTH = 160;

export function PageList() {
  const cache = useThumbnailCache();

  return (
    <CardSurface
      columnCount={1}
      rowHeight={ROW_HEIGHT}
      viewMode="list"
      renderCard={(card, thumbnailWidth, selected) => (
        <PageListRow {...toCardViewProps(card, cache, thumbnailWidth, selected)} />
      )}
      thumbnailWidth={THUMBNAIL_WIDTH}
    />
  );
}
