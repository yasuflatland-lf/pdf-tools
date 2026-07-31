import type { SourceFileDto } from "../bindings/SourceFileDto";
import { removeSlots } from "../lib/tauri-api";
import { useShortcuts } from "../lib/useShortcuts";
import { usePlanStore } from "../store/plan-store";
import { useUiStore } from "../store/ui-store";
import { useSnapshotSync } from "../store/useSnapshotSync";
import { DropZone } from "./DropZone";
import { SourceErrorCard } from "./PageCard";
import { PageGrid } from "./PageGrid";
import { PageList } from "./PageList";
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
  useSnapshotSync();
  const sources = usePlanStore((state) => state.sources);
  const slotCount = usePlanStore((state) => state.slots.length);
  const viewMode = useUiStore((state) => state.viewMode);
  const unusableSources = sources.filter((source) => source.status.kind !== "ready");

  // These stay on the shell so Escape works even after the grid unmounts with
  // an empty plan. History and focus register their own disjoint actions.
  useShortcuts({
    "select-all": () => {
      useUiStore.getState().selectSlots(usePlanStore.getState().slots.map((slot) => slot.id));
      return true;
    },
    "clear-selection": () => {
      useUiStore.getState().clearSelection();
      return true;
    },
    "remove-selected": () => {
      const selected = useUiStore.getState().selectedSlots;
      if (selected.size === 0) {
        return false;
      }

      void removeSlots([...selected])
        .then((snapshot) => usePlanStore.getState().setSnapshot(snapshot))
        .catch((error: unknown) => console.error("remove failed", error));
      return true;
    },
  });

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-slate-950 text-slate-100">
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
