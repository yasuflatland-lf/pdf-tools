import type { ReactElement, ReactNode } from "react";
import type { SourceStatusDto } from "../bindings/SourceStatusDto";
import type { ThumbnailCache } from "../lib/thumbnail-cache";
import { countLabel } from "../lib/format";
import { ErrorBadge } from "./ErrorBadge";
import { useThumbnail } from "./useThumbnail";

interface PageCardProps {
  cache: ThumbnailCache;
  collapsed: boolean;
  fileName: string;
  pageCount: number;
  pageNumber: number;
  slotId: number;
  thumbnailWidth: number;
  selected?: boolean;
}

/**
 * The shared card chrome: a fixed-height preview area above the file name and a
 * caption. Every card in the window keeps the same silhouette, so a source that
 * cannot be merged reads as one of the cards rather than as a stray notice.
 */
function CardFrame({
  caption,
  className,
  fileName,
  preview,
}: {
  caption: ReactNode;
  className: string;
  fileName: string;
  preview: ReactNode;
}): ReactElement {
  return (
    <article className={`overflow-hidden rounded-xl border bg-slate-900 ${className}`}>
      <div className="grid h-60 place-items-center bg-slate-800/70">{preview}</div>
      <div className="px-3 py-3">
        <p className="truncate font-medium text-slate-100" title={fileName}>
          {fileName}
        </p>
        {caption}
      </div>
    </article>
  );
}

export function PageCard({
  cache,
  collapsed,
  fileName,
  pageCount,
  pageNumber,
  slotId,
  thumbnailWidth,
  selected,
}: PageCardProps) {
  const { url: thumbnailUrl, failed } = useThumbnail(cache, slotId, thumbnailWidth);

  return (
    <CardFrame
      // The option role, the selection state and the DOM focus live on the drag
      // wrapper in `SortableCard`, which is the node the grid's listbox owns.
      // This element only renders the selection.
      className={selected ? "border-sky-400 ring-2 ring-sky-400/60" : "border-slate-800"}
      fileName={fileName}
      preview={
        thumbnailUrl ? (
          <img
            className="h-full w-full object-contain"
            src={thumbnailUrl}
            alt={`Thumbnail for ${fileName}`}
          />
        ) : (
          // A page-shaped placeholder keeps the card the same size before the
          // thumbnail arrives, so a scrolling grid never reflows.
          <div className="flex flex-col items-center">
            <div
              className="h-20 w-16 rounded border border-slate-600 bg-slate-700"
              role="img"
              aria-label={failed ? "Thumbnail unavailable" : "Loading thumbnail"}
            />
            {failed && (
              <p className="mt-2 rounded-md border border-amber-700/60 bg-amber-950/50 px-2 py-1 text-xs text-amber-100">
                サムネイルを表示できません
              </p>
            )}
          </div>
        )
      }
      caption={
        <p className="mt-1 text-sm text-slate-400">
          {collapsed ? countLabel(pageCount, "page") : `Page ${pageNumber}`}
        </p>
      }
    />
  );
}

/**
 * A source that could not be read contributes no page slots, so it has no
 * thumbnail to request and never reaches the page grid. It still gets a card:
 * dropping a file and seeing nothing appear looks like the file was lost. The
 * card is dimmed and badged so that it reads as excluded at a glance.
 */
export function SourceErrorCard({
  fileName,
  status,
}: {
  fileName: string;
  status: SourceStatusDto;
}): ReactElement {
  return (
    <CardFrame
      className="border-amber-700/60 opacity-60"
      fileName={fileName}
      preview={
        <div className="flex h-24 w-20 flex-col items-center justify-center gap-2 rounded border border-amber-700/60 bg-amber-950/40 text-center text-xs font-medium text-amber-100">
          <span className="text-2xl" aria-hidden="true">
            !
          </span>
          <span>結合対象外</span>
        </div>
      }
      caption={<ErrorBadge status={status} />}
    />
  );
}
