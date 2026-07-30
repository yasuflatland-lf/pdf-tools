import type { SourceFileDto } from "../bindings/SourceFileDto";
import { usePlanStore } from "../store/plan-store";
import { DropZone } from "./DropZone";
import { SourceErrorCard } from "./PageCard";
import { PageGrid } from "./PageGrid";
import { Toolbar } from "./Toolbar";

/**
 * Files that contribute no pages would otherwise vanish from the window, since
 * the grid only shows slots. Keeping them visible makes their exclusion and the
 * reason clear instead of making an imported file appear to be lost.
 */
function UnusableSources({ sources }: { sources: SourceFileDto[] }) {
  return (
    <ul
      className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4"
      aria-label="Unusable source files"
    >
      {sources.map((source) => (
        <li key={source.id}>
          <SourceErrorCard fileName={source.file_name} status={source.status} />
        </li>
      ))}
    </ul>
  );
}

export function AppShell() {
  const sources = usePlanStore((state) => state.sources);
  const slotCount = usePlanStore((state) => state.slots.length);
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
            ) : (
              <PageGrid />
            )}
          </div>
        </main>
      </DropZone>
    </div>
  );
}
