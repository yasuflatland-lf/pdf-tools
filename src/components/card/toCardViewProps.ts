import type { ThumbnailCache } from "../../lib/thumbnail-cache";
import { usePlanStore } from "../../store/plan-store";
import { useUiStore } from "../../store/ui-store";
import type { DisplayCard } from "../usePageCards";
import type { CardViewProps } from "./CardProps";

/**
 * Projects one `DisplayCard` onto the props a card shape takes. Both views
 * derive them identically -- the page number is one-based, the toggle is only
 * offered for a foldable run, and a rotation acts on every slot the card
 * stands for, not just its first.
 */
export function toCardViewProps(
  card: DisplayCard,
  cache: ThumbnailCache,
  thumbnailWidth: number,
  selected: boolean,
): CardViewProps {
  return {
    cache,
    collapsed: card.collapsed,
    fileName: card.fileName,
    pageCount: card.pageCount,
    pageNumber: card.slot.page + 1,
    rotation: card.slot.rotation,
    selected,
    slotId: card.slot.id,
    thumbnailWidth,
    onToggle: card.collapsible
      ? () => useUiStore.getState().toggleExpanded(card.slot.source)
      : undefined,
    onRotate: (delta) => {
      void usePlanStore
        .getState()
        .rotate(card.slotIds, delta)
        .catch((error: unknown) => console.error("rotate failed", error));
    },
  };
}
