import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useCallback, useEffect } from "react";
import { countLabel } from "../lib/format";
import { resolveShortcut } from "../lib/keyboard";
import { redo, undo } from "../lib/tauri-api";
import { usePlanStore } from "../store/plan-store";
import { useUiStore } from "../store/ui-store";
import { ErrorDialog } from "./ErrorDialog";
import { MergeProgressLine } from "./MergeProgressLine";
import { useMerge } from "./useMerge";
import { ViewToggle } from "./ViewToggle";

async function showInFolder(dest: string): Promise<void> {
  try {
    await revealItemInDir(dest);
  } catch (error) {
    console.error("reveal item failed", error);
  }
}

export function Toolbar() {
  const fileCount = usePlanStore((state) => state.sources.length);
  const pageCount = usePlanStore((state) => state.slots.length);
  const canUndo = usePlanStore((state) => state.canUndo);
  const canRedo = usePlanStore((state) => state.canRedo);
  const modalOpen = useUiStore((state) => state.modalOpen);
  const { isMerging, progress, result, failure, start } = useMerge();

  const performUndo = useCallback(async () => {
    try {
      usePlanStore.getState().setSnapshot(await undo());
    } catch (error) {
      console.error("undo failed", error);
    }
  }, []);

  const performRedo = useCallback(async () => {
    try {
      usePlanStore.getState().setSnapshot(await redo());
    } catch (error) {
      console.error("redo failed", error);
    }
  }, []);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      // A modal owns the keyboard while it is open; nothing here may reach the
      // document behind it.
      if (useUiStore.getState().modalOpen) {
        return;
      }

      const action = resolveShortcut(event);
      if (action !== "undo" && action !== "redo") {
        return;
      }
      if (event.defaultPrevented) {
        return;
      }

      if (isMerging || (action === "redo" ? !canRedo : !canUndo)) {
        return;
      }

      event.preventDefault();
      void (action === "redo" ? performRedo() : performUndo());
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [canRedo, canUndo, isMerging, performRedo, performUndo]);

  return (
    <div className="relative flex items-center gap-3 border-y border-slate-800 bg-slate-900/80 px-6 py-3 text-sm text-slate-300">
      <span>{countLabel(fileCount, "file")}</span>
      <span aria-hidden="true" className="text-slate-600">
        /
      </span>
      <span>{countLabel(pageCount, "page")}</span>
      <button
        className="rounded-md border border-slate-700 px-3 py-1.5 hover:bg-slate-800 disabled:cursor-not-allowed disabled:text-slate-600 disabled:hover:bg-transparent"
        disabled={!canUndo || isMerging}
        onClick={() => void performUndo()}
        type="button"
      >
        Undo
      </button>
      <button
        className="rounded-md border border-slate-700 px-3 py-1.5 hover:bg-slate-800 disabled:cursor-not-allowed disabled:text-slate-600 disabled:hover:bg-transparent"
        disabled={!canRedo || isMerging}
        onClick={() => void performRedo()}
        type="button"
      >
        Redo
      </button>
      <ViewToggle />
      <div className="ml-auto flex items-center gap-3">
        {result && (
          <>
            <span>
              {result.dest.split(/[\\/]/).pop()} · {countLabel(result.report.page_count, "page")}
            </span>
            <button
              className="rounded-md border border-slate-700 px-3 py-1.5 hover:bg-slate-800"
              onClick={() => void showInFolder(result.dest)}
              type="button"
            >
              Show in folder
            </button>
          </>
        )}
        {failure && <span className="text-red-400">Merge failed</span>}
        <button
          className="rounded-md bg-sky-600 px-4 py-1.5 font-medium text-white hover:bg-sky-500 disabled:cursor-not-allowed disabled:bg-slate-700 disabled:text-slate-400"
          disabled={pageCount === 0 || isMerging}
          onClick={() => void start()}
          type="button"
        >
          Merge
        </button>
      </div>
      {isMerging && (
        <MergeProgressLine done={progress.done} label="Merge progress" total={progress.total} />
      )}
      {failure && modalOpen && (
        <ErrorDialog
          files={failure.files}
          message={failure.message}
          onClose={() => useUiStore.getState().setModalOpen(false)}
        />
      )}
    </div>
  );
}
