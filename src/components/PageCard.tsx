import { useEffect, useRef, useState } from "react";
import type { ThumbnailCache } from "../lib/thumbnail-cache";
import { countLabel } from "../lib/format";

interface PageCardProps {
  cache: ThumbnailCache;
  collapsed: boolean;
  fileName: string;
  pageCount: number;
  pageNumber: number;
  slotId: number;
  thumbnailWidth: number;
  focused?: boolean;
  selected?: boolean;
}

export function PageCard({
  cache,
  collapsed,
  fileName,
  pageCount,
  pageNumber,
  slotId,
  thumbnailWidth,
  focused,
  selected,
}: PageCardProps) {
  const cardRef = useRef<HTMLElement>(null);
  const [thumbnailUrl, setThumbnailUrl] = useState<string>();
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let active = true;
    setThumbnailUrl(undefined);
    setFailed(false);

    void cache
      .get(slotId, thumbnailWidth)
      .then((url) => {
        if (active && url) {
          setThumbnailUrl(url);
        }
      })
      .catch(() => {
        if (active) {
          setFailed(true);
        }
      });

    return () => {
      active = false;
    };
  }, [cache, slotId, thumbnailWidth]);

  // Arrow navigation only moves an index; the card the index lands on is what
  // takes the DOM focus. A card scrolled into view by the virtualizer mounts
  // already focused, so this also covers the rows that were not rendered yet.
  useEffect(() => {
    if (focused) {
      cardRef.current?.focus();
    }
  }, [focused]);

  return (
    <article
      ref={cardRef}
      role="option"
      aria-selected={selected === true}
      // A roving tabindex: only the focused card is a tab stop, so Tab crosses
      // the grid instead of walking through every page in the document.
      tabIndex={focused ? 0 : -1}
      className={`overflow-hidden rounded-xl border bg-slate-900 ${
        selected ? "border-sky-400 ring-2 ring-sky-400/60" : "border-slate-800"
      }`}
    >
      <div className="grid h-60 place-items-center bg-slate-800/70">
        {thumbnailUrl ? (
          <img
            className="h-full w-full object-contain"
            src={thumbnailUrl}
            alt={`Thumbnail for ${fileName}`}
          />
        ) : (
          // A page-shaped placeholder keeps the card the same size before the
          // thumbnail arrives, so a scrolling grid never reflows.
          <div
            className="h-20 w-16 rounded border border-slate-600 bg-slate-700"
            role="img"
            aria-label={failed ? "Thumbnail unavailable" : "Loading thumbnail"}
          />
        )}
      </div>
      <div className="px-3 py-3">
        <p className="truncate font-medium text-slate-100" title={fileName}>
          {fileName}
        </p>
        <p className="mt-1 text-sm text-slate-400">
          {collapsed ? countLabel(pageCount, "page") : `Page ${pageNumber}`}
        </p>
      </div>
    </article>
  );
}
