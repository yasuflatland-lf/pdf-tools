interface ProgressBarProps {
  done: number;
  total: number;
  label: string;
}

export function ProgressBar({ done, total, label }: ProgressBarProps) {
  const percent = total <= 0 ? 0 : Math.min(100, Math.max(0, Math.round((done / total) * 100)));

  return (
    <div
      aria-label={label}
      aria-valuemax={100}
      aria-valuemin={0}
      aria-valuenow={percent}
      className="h-2 w-32 overflow-hidden rounded-full bg-slate-800"
      role="progressbar"
    >
      <div className="h-full rounded-full bg-sky-500" style={{ width: `${percent}%` }} />
    </div>
  );
}
