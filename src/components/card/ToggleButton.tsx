import { CardControl } from "./CardControl";

interface ToggleButtonProps {
  collapsed: boolean;
  fileName: string;
  onToggle: () => void;
  /** Extra positioning for the grid, which floats the control over the preview. */
  className?: string;
}

/** Expands or collapses one card's run. */
export function ToggleButton({ collapsed, fileName, onToggle, className = "" }: ToggleButtonProps) {
  const label = collapsed ? "Expand" : "Collapse";

  return (
    <CardControl label={`${label} ${fileName}`} onPress={onToggle} className={className}>
      {label}
    </CardControl>
  );
}
