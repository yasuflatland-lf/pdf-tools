import { RotateCcw, RotateCw } from "lucide-react";
import type { ReactElement, ReactNode } from "react";
import type { SourceStatusDto } from "../bindings/SourceStatusDto";
import { countLabel } from "../lib/format";
import { ErrorBadge } from "./ErrorBadge";
import { CardControl, CardIconControl } from "./card/CardControl";
import type { CardViewProps } from "./card/CardProps";
import { Notice } from "./card/Notice";
import { ThumbnailFrame } from "./card/ThumbnailFrame";

/** Left before right, which is the order the toolbar draws them in too. */
const ROTATIONS = [
  { delta: -1, icon: RotateCcw, direction: "left" },
  { delta: 1, icon: RotateCw, direction: "right" },
] as const;

type Props = CardViewProps;

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
  rotation,
  slotId,
  thumbnailWidth,
  onToggle: _onToggle, // GroupCard positions the grid toggle outside PageCard.
  onRotate,
  selected,
}: Props) {
  const [thumbnail, failed] = ThumbnailFrame({
    cache,
    fileName,
    rotation,
    slotId,
    thumbnailWidth,
    placeholderClassName: "h-20 w-16",
  });

  return (
    <CardFrame
      // The option role, the selection state and the DOM focus live on the drag
      // wrapper in `SortableCard`, which is the node the grid's listbox owns.
      // This element only renders the selection.
      className={selected ? "border-sky-400 ring-2 ring-sky-400/60" : "border-slate-800"}
      fileName={fileName}
      preview={
        <div className="group relative h-full w-full">
          <div className="flex h-full w-full flex-col items-center justify-center">
            {thumbnail}
            {failed && <Notice>Thumbnail unavailable</Notice>}
          </div>
          {onRotate && (
            <div className="absolute top-2 right-2 z-20 flex gap-1 opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100">
              {ROTATIONS.map(({ delta, icon, direction }) => (
                <CardIconControl
                  key={direction}
                  icon={icon}
                  label={`Rotate ${direction} ${fileName}`}
                  onPress={() => onRotate(delta)}
                  title={`Rotate ${direction}`}
                />
              ))}
            </div>
          )}
        </div>
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
  onDismiss,
  status,
}: {
  fileName: string;
  onDismiss: () => void;
  status: SourceStatusDto;
}): ReactElement {
  return (
    <CardFrame
      className="border-amber-700/60 opacity-60"
      fileName={fileName}
      preview={
        <div className="flex items-center gap-3">
          <div className="flex h-24 w-20 flex-col items-center justify-center gap-2 rounded border border-amber-700/60 bg-amber-950/40 text-center text-xs font-medium text-amber-100">
            <span className="text-2xl" aria-hidden="true">
              !
            </span>
            <span>Not merged</span>
          </div>
          <CardControl label={`Remove ${fileName}`} onPress={onDismiss}>
            Remove
          </CardControl>
        </div>
      }
      caption={<ErrorBadge status={status} />}
    />
  );
}
