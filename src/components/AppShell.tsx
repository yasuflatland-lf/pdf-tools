import type { SourceFileDto } from "../bindings/SourceFileDto";
import { statusLabel } from "../lib/format";
import { usePlanStore } from "../store/plan-store";
import { useUiStore } from "../store/ui-store";
import { DropZone } from "./DropZone";
import { PageGrid } from "./PageGrid";
import { PageList } from "./PageList";
import { Toolbar } from "./Toolbar";

/**
 * Files that contribute no pages would otherwise vanish from the window, since
 * the grid only shows slots. Proper error reporting is Task 29; until then this
 * at least names the files that were dropped and did nothing.
 */
function UnusableSources({ sources }: { sources: SourceFileDto[] }) {
  return (
    <ul className="space-y-2" aria-label="Unusable source files">
      {sources.map((source) => (
        <li
          key={source.id}
          className="rounded-lg border border-amber-700/60 bg-amber-950/40 px-4 py-2 text-sm"
        >
          <span className="font-medium text-amber-100">{source.file_name}</span>
          <span className="text-amber-200/80"> · {statusLabel(source.status)}</span>
        </li>
      ))}
    </ul>
  );
}

export function AppShell() {
  const sources = usePlanStore((state) => state.sources);
  const slotCount = usePlanStore((state) => state.slots.length);
  const viewMode = useUiStore((state) => state.viewMode);
  const unusableSources = sources.filter((source) => source.status.kind !== "ready");

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-slate-950 text-slate-100">
      <header className="bg-slate-950 px-6 py-5">
        <h1 className="text-2xl font-semibold tracking-tight">PDF Tools</h1>
        <p className="mt-1 text-sm text-slate-400">Combine PDFs and images into a single PDF.</p>
      </header>

      <Toolbar />

      <DropZone>
        <main className="flex h-full flex-col gap-4 overflow-hidden px-6 py-6">
          {unusableSources.length > 0 && <UnusableSources sources={unusableSources} />}

          <div className="min-h-0 flex-1">
            {slotCount === 0 ? (
              <div className="grid min-h-48 place-items-center rounded-xl border border-dashed border-slate-700 text-center">
                <div>
                  <p className="font-medium text-slate-200">Drop PDFs or images here</p>
                  <p className="mt-1 text-sm text-slate-500">
                    Source files will appear in this document.
                  </p>
                </div>
              </div>
            ) : viewMode === "grid" ? (
              <PageGrid />
            ) : (
              <PageList />
            )}
          </div>
        </main>
      </DropZone>
    </div>
  );
}
