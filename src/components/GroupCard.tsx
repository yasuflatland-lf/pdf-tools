import type { ComponentProps } from "react";
import { PageCard } from "./PageCard";

type GroupCardProps = ComponentProps<typeof PageCard> & {
  onToggle?: () => void;
};

export function GroupCard({ onToggle, ...pageCardProps }: GroupCardProps) {
  const { collapsed, fileName } = pageCardProps;

  return (
    <div className="relative">
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
