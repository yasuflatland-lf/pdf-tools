import { useEffect, useState } from "react";
import type { SourceStatusDto } from "../bindings/SourceStatusDto";
import { countLabel, statusLabel } from "../lib/format";
import type { ThumbnailCache } from "../lib/thumbnail-cache";

interface PageListRowProps {
  cache: ThumbnailCache;
  collapsed: boolean;
  fileName: string;
  onToggle?: () => void;
  pageCount: number;
  pageNumber: number;
  slotId: number;
  status?: SourceStatusDto;
  thumbnailWidth: number;
}

export function PageListRow({
  cache,
  collapsed,
  fileName,
  onToggle,
  pageCount,
  pageNumber,
  slotId,
  status,
  thumbnailWidth,
}: PageListRowProps) {
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
    <article className="flex h-24 items-center gap-4 overflow-hidden rounded-xl border border-slate-800 bg-slate-900 px-3 py-2">
      <div className="grid h-20 w-16 shrink-0 place-items-center overflow-hidden rounded bg-slate-800/70">
        {thumbnailUrl ? (
          <img
            className="h-full w-full object-contain"
            src={thumbnailUrl}
            alt={`Thumbnail for ${fileName}`}
          />
        ) : (
          <div
            className="h-14 w-11 rounded border border-slate-600 bg-slate-700"
            role="img"
            aria-label={failed ? "Thumbnail unavailable" : "Loading thumbnail"}
          />
        )}
      </div>
      <div className="min-w-0 flex-1">
        <p className="truncate font-medium text-slate-100" title={fileName}>
          {fileName}
        </p>
        <p className="mt-1 text-sm text-slate-400">
          {collapsed ? countLabel(pageCount, "page") : `Page ${pageNumber}`}
        </p>
      </div>
      <p className="shrink-0 text-sm text-slate-400">
        {status ? statusLabel(status) : "Unknown status"}
      </p>
      {onToggle &&
        (collapsed ? (
          <button
            aria-label={`Expand ${fileName}`}
            className="shrink-0 rounded-md border border-slate-600 bg-slate-900 px-2 py-1 text-xs font-medium text-slate-200 shadow hover:bg-slate-800 focus-visible:ring-2 focus-visible:ring-sky-400 focus-visible:outline-none"
            onClick={onToggle}
            onKeyDown={(event) => event.stopPropagation()}
            type="button"
          >
            Expand
          </button>
        ) : (
          <button
            aria-label={`Collapse ${fileName}`}
            className="shrink-0 rounded-md border border-slate-600 bg-slate-900 px-2 py-1 text-xs font-medium text-slate-200 shadow hover:bg-slate-800 focus-visible:ring-2 focus-visible:ring-sky-400 focus-visible:outline-none"
            onClick={onToggle}
            onKeyDown={(event) => event.stopPropagation()}
            type="button"
          >
            Collapse
          </button>
        ))}
    </article>
  );
}
