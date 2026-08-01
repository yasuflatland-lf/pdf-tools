interface ToggleButtonProps {
  collapsed: boolean;
  fileName: string;
  onToggle: () => void;
  /** Extra positioning for the grid, which floats the control over the preview. */
  className?: string;
}

/**
 * Expands or collapses one card's run. The keydown is stopped so the control
 * never steers the grid behind it.
 */
export function ToggleButton({ collapsed, fileName, onToggle, className = "" }: ToggleButtonProps) {
  const label = collapsed ? "Expand" : "Collapse";

  return (
    <button
      aria-label={`${label} ${fileName}`}
      className={`rounded-md border border-slate-600 bg-slate-900/90 px-2 py-1 text-xs font-medium text-slate-200 shadow hover:bg-slate-800 focus-visible:ring-2 focus-visible:ring-sky-400 focus-visible:outline-none ${className}`}
      onClick={onToggle}
      onKeyDown={(event) => event.stopPropagation()}
      type="button"
    >
      {label}
    </button>
  );
}
