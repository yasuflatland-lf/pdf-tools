interface MergeProgressLineProps {
  done: number;
  total: number;
  label: string;
}

/**
 * Merge progress rides the bottom edge of the toolbar. The row's centre group is
 * positioned against the window midpoint, so a progress bar that sat inside the
 * row would push into it as soon as it appeared.
 */
export function MergeProgressLine({ done, total, label }: MergeProgressLineProps) {
  const percent = total <= 0 ? 0 : Math.min(100, Math.max(0, Math.round((done / total) * 100)));

  return (
    <div
      aria-label={label}
      aria-valuemax={100}
      aria-valuemin={0}
      aria-valuenow={percent}
      className="absolute inset-x-0 bottom-0 h-0.5 bg-slate-950"
      role="progressbar"
    >
      <div
        className="h-full bg-sky-400 transition-[width] duration-150"
        style={{ width: `${percent}%` }}
      />
    </div>
  );
}
