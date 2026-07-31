import { LayoutGrid, List, type LucideIcon } from "lucide-react";
import { useUiStore, type ViewMode } from "../store/ui-store";

const MODES: { mode: ViewMode; label: string; icon: LucideIcon }[] = [
  { mode: "grid", label: "Grid view", icon: LayoutGrid },
  { mode: "list", label: "List view", icon: List },
];

/**
 * The selected mode is a filled thumb inside an enclosure. A bare fill would be
 * indistinguishable from a hover, which is exactly the confusion the previous
 * pair of bordered text buttons produced.
 */
function thumbClass(active: boolean): string {
  const state = active
    ? "bg-slate-700 text-slate-50 shadow-sm"
    : "text-slate-400 hover:text-slate-200";
  return `grid h-6.5 w-8 place-items-center rounded-md focus-visible:outline-2 focus-visible:outline-sky-500 ${state}`;
}

export function ViewToggle() {
  const viewMode = useUiStore((state) => state.viewMode);
  const setViewMode = useUiStore((state) => state.setViewMode);

  return (
    <div
      aria-label="View mode"
      className="flex items-center gap-0.5 rounded-lg border border-slate-800 bg-slate-950 p-0.5"
      role="group"
    >
      {MODES.map(({ mode, label, icon: Icon }) => (
        <button
          aria-label={label}
          aria-pressed={viewMode === mode}
          className={thumbClass(viewMode === mode)}
          key={mode}
          onClick={() => setViewMode(mode)}
          title={label}
          type="button"
        >
          <Icon aria-hidden="true" size={16} strokeWidth={1.8} />
        </button>
      ))}
    </div>
  );
}
