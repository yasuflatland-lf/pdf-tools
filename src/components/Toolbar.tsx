import { countLabel } from "../lib/format";
import { usePlanStore } from "../store/plan-store";

export function Toolbar() {
  const fileCount = usePlanStore((state) => state.sources.length);
  const pageCount = usePlanStore((state) => state.slots.length);

  return (
    <div className="flex items-center gap-3 border-y border-slate-800 bg-slate-900/80 px-6 py-3 text-sm text-slate-300">
      <span>{countLabel(fileCount, "file")}</span>
      <span aria-hidden="true" className="text-slate-600">
        /
      </span>
      <span>{countLabel(pageCount, "page")}</span>
    </div>
  );
}
