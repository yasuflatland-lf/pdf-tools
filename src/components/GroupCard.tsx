import type { ComponentProps } from "react";
import { PageCard } from "./PageCard";

type GroupCardProps = ComponentProps<typeof PageCard> & {
  onToggle?: () => void;
};

export function GroupCard({ onToggle, ...pageCardProps }: GroupCardProps) {
  const { collapsed, fileName } = pageCardProps;

  return (
    // The collapsed card's Expand control is a full-preview overlay, so the
    // pointer never rests inside `PageCard`'s own subtree and the hover marker
    // there can never match. Marking the element that holds both the overlay
    // and the card reveals the rotate buttons wherever on the card the pointer
    // is -- `group-hover` matches any marked ancestor, so the inner marker
    // keeps working for a card with no overlay.
    <div className="group relative">
      <PageCard {...pageCardProps} />
      {onToggle &&
        (collapsed ? (
          <button
            aria-label={`Expand ${fileName}`}
            className="absolute inset-x-0 top-0 flex h-60 items-start justify-start rounded-t-xl p-2 text-xs font-medium text-slate-200 focus-visible:ring-2 focus-visible:ring-sky-400 focus-visible:outline-none"
            onClick={onToggle}
            onKeyDown={(event) => event.stopPropagation()}
            type="button"
          >
            <span className="rounded-md border border-slate-600 bg-slate-900/90 px-2 py-1 shadow">
              Expand
            </span>
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
