import type { LucideIcon } from "lucide-react";

interface ToolbarIconButtonProps {
  icon: LucideIcon;
  /** The accessible name, and the first half of the tooltip. */
  label: string;
  /** Printable shortcut, appended to the tooltip in parentheses. */
  shortcut?: string;
  disabled?: boolean;
  onClick: () => void;
}

/**
 * A toolbar action with no visible text. The name and the tooltip are settled
 * here rather than at each call site, because a labelless icon that also lacks
 * an accessible name is unusable rather than merely terse.
 */
export function ToolbarIconButton({
  icon: Icon,
  label,
  shortcut,
  disabled = false,
  onClick,
}: ToolbarIconButtonProps) {
  return (
    <button
      aria-label={label}
      className="grid h-8 w-8 place-items-center rounded-md text-slate-300 hover:bg-slate-800 focus-visible:outline-2 focus-visible:outline-sky-500 disabled:cursor-not-allowed disabled:text-slate-600 disabled:hover:bg-transparent"
      disabled={disabled}
      onClick={onClick}
      title={shortcut === undefined ? label : `${label} (${shortcut})`}
      type="button"
    >
      <Icon aria-hidden="true" size={17} strokeWidth={1.8} />
    </button>
  );
}
