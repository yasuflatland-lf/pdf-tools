import { useEffect } from "react";
import type { SourceFileDto } from "../bindings/SourceFileDto";
import { resolveShortcut } from "../lib/keyboard";
import { removeSlots } from "../lib/tauri-api";
import { usePlanStore } from "../store/plan-store";
import { useUiStore } from "../store/ui-store";
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
  const sources = usePlanStore((state) => state.sources);
  const slotCount = usePlanStore((state) => state.slots.length);
  const viewMode = useUiStore((state) => state.viewMode);
  const unusableSources = sources.filter((source) => source.status.kind !== "ready");

  /**
   * The document-wide shortcuts. They live on the shell rather than on the grid
   * because the grid unmounts once the plan is empty, and Escape has to keep
   * working there. Undo and redo stay in the toolbar, which owns the flags that
   * gate them; handling them here as well would run each of them twice.
   */
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      // A modal owns the keyboard while it is open; nothing here may reach the
      // document behind it.
      if (useUiStore.getState().modalOpen) {
        return;
      }

      // A card drag consumes its own keys; nothing already handled is a shortcut.
      if (event.defaultPrevented) {
        return;
      }

      const action = resolveShortcut(event);
      if (action === "select-all") {
        event.preventDefault();
        useUiStore.getState().selectSlots(usePlanStore.getState().slots.map((slot) => slot.id));
      } else if (action === "clear-selection") {
        event.preventDefault();
        useUiStore.getState().clearSelection();
      } else if (action === "remove-selected") {
        const selected = useUiStore.getState().selectedSlots;
        if (selected.size === 0) {
          return;
        }

        event.preventDefault();
        void removeSlots([...selected])
          .then((snapshot) => usePlanStore.getState().setSnapshot(snapshot))
          .catch((error: unknown) => console.error("remove failed", error));
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

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
