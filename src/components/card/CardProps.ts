import type { ThumbnailCache } from "../../lib/thumbnail-cache";

/**
 * What every card shape is given, in either view. A card stands for one slot,
 * or for the whole run a collapsed group folds -- `pageCount` reports how many
 * pages are actually present, which is not `source.page_count`.
 */
export interface CardViewProps {
  cache: ThumbnailCache;
  collapsed: boolean;
  fileName: string;
  pageCount: number;
  pageNumber: number;
  rotation: number;
  selected: boolean;
  slotId: number;
  thumbnailWidth: number;
  /** Absent when the run is a single page, which has nothing to fold. */
  onToggle?: () => void;
  /**
   * Absent in list view, which has no room for the hover controls. The toolbar
   * and the keyboard reach the same operation for a selected card either way.
   */
  onRotate?: (delta: number) => void;
}
