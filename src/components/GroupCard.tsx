import type { ComponentProps } from "react";
import { PageCard } from "./PageCard";

type GroupCardProps = ComponentProps<typeof PageCard> & {
  onToggle?: () => void;
};

export function GroupCard({ onToggle, ...pageCardProps }: GroupCardProps) {
  const { collapsed, fileName } = pageCardProps;

  return (
    // Both toggles sit outside `PageCard`, so the pointer can rest on one of
    // them without entering the hover marker inside the card. Marking the
    // element that holds the card and the toggle together reveals the rotate
    // buttons wherever on the card the pointer is -- `group-hover` matches any
    // marked ancestor, so the inner marker keeps working for a card with no
    // toggle at all.
    <div className="group relative">
      <PageCard {...pageCardProps} />
      {onToggle &&
        (collapsed ? (
          // A badge rather than a blanket over the preview: the preview is most
          // of the card, and a control covering it would take every click meant
          // to select the card -- including the Shift-click that ends a range.
          <button
            aria-label={`Expand ${fileName}`}
            className="absolute top-2 left-2 rounded-md border border-slate-600 bg-slate-900/90 px-2 py-1 text-xs font-medium text-slate-200 shadow hover:bg-slate-800 focus-visible:ring-2 focus-visible:ring-sky-400 focus-visible:outline-none"
            onClick={onToggle}
            onKeyDown={(event) => event.stopPropagation()}
            type="button"
          >
            Expand
          </button>
        ) : (
          <button
            aria-label={`Collapse ${fileName}`}
            className="absolute top-2 left-2 rounded-md border border-slate-600 bg-slate-900/90 px-2 py-1 text-xs font-medium text-slate-200 shadow hover:bg-slate-800 focus-visible:ring-2 focus-visible:ring-sky-400 focus-visible:outline-none"
            onClick={onToggle}
            onKeyDown={(event) => event.stopPropagation()}
            type="button"
          >
            Collapse
          </button>
        ))}
    </div>
  );
}
