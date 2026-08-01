import { countLabel } from "../lib/format";
import type { ThumbnailCache } from "../lib/thumbnail-cache";
import { Notice } from "./card/Notice";
import { ThumbnailFrame } from "./card/ThumbnailFrame";
import { ToggleButton } from "./card/ToggleButton";

interface PageListRowProps {
  cache: ThumbnailCache;
  collapsed: boolean;
  fileName: string;
  onToggle?: () => void;
  pageCount: number;
  pageNumber: number;
  rotation: number;
  slotId: number;
  thumbnailWidth: number;
}

export function PageListRow({
  cache,
  collapsed,
  fileName,
  onToggle,
  pageCount,
  pageNumber,
  rotation,
  slotId,
  thumbnailWidth,
}: PageListRowProps) {
  const [thumbnail, failed] = ThumbnailFrame({
    cache,
    fileName,
    rotation,
    slotId,
    thumbnailWidth,
    placeholderClassName: "h-14 w-11",
  });

  return (
    <article className="flex h-24 items-center gap-4 overflow-hidden rounded-xl border border-slate-800 bg-slate-900 px-3 py-2">
      <div className="grid h-20 w-16 shrink-0 place-items-center overflow-hidden rounded bg-slate-800/70">
        {thumbnail}
      </div>
      {failed && <Notice>Thumbnail unavailable</Notice>}
      <div className="min-w-0 flex-1">
        <p className="truncate font-medium text-slate-100" title={fileName}>
          {fileName}
        </p>
        <p className="mt-1 text-sm text-slate-400">
          {collapsed ? countLabel(pageCount, "page") : `Page ${pageNumber}`}
        </p>
      </div>
      {onToggle && (
        <ToggleButton
          collapsed={collapsed}
          fileName={fileName}
          onToggle={onToggle}
          className="shrink-0"
        />
      )}
    </article>
  );
}
