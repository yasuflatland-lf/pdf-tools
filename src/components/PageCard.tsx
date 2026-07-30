import { useEffect, useState } from "react";
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
}

export function PageCard({
  cache,
  collapsed,
  fileName,
  pageCount,
  pageNumber,
  slotId,
  thumbnailWidth,
}: PageCardProps) {
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

  return (
    <article className="overflow-hidden rounded-xl border border-slate-800 bg-slate-900">
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
