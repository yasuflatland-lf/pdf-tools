import type { ReactElement } from "react";
import type { ThumbnailCache } from "../../lib/thumbnail-cache";
import { RotatedThumbnail } from "../RotatedThumbnail";
import { useThumbnail } from "../useThumbnail";

interface ThumbnailFrameProps {
  cache: ThumbnailCache;
  fileName: string;
  rotation: number;
  slotId: number;
  thumbnailWidth: number;
  /** Tailwind size classes for the placeholder, which differs between the views. */
  placeholderClassName: string;
}

/**
 * One slot's preview, and the page-shaped placeholder that holds its size
 * before the bytes arrive. Both card shapes request a thumbnail exactly this
 * way, and a scrolling view must not reflow when one lands.
 *
 * Called as a plain function rather than rendered as an element, because the
 * caller needs `failed` as well: the grid stacks its `Notice` under the
 * placeholder while the list puts it beside the frame, so placement cannot
 * live here. The call is unconditional in both callers, which keeps
 * `useThumbnail` in a stable position in their hook order.
 */
export function ThumbnailFrame({
  cache,
  fileName,
  rotation,
  slotId,
  thumbnailWidth,
  placeholderClassName,
}: ThumbnailFrameProps): [ReactElement, boolean] {
  const { url: thumbnailUrl, failed } = useThumbnail(cache, slotId, thumbnailWidth);
  const thumbnail = thumbnailUrl ? (
    <RotatedThumbnail alt={`Thumbnail for ${fileName}`} rotation={rotation} src={thumbnailUrl} />
  ) : (
    <div
      className={`${placeholderClassName} rounded border border-slate-600 bg-slate-700`}
      role="img"
      aria-label={failed ? "Thumbnail unavailable" : "Loading thumbnail"}
    />
  );

  return [thumbnail, failed];
}
