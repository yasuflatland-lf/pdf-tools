import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { FolderOpen, Redo2, Undo2 } from "lucide-react";
import { useCallback, useEffect } from "react";
import { countLabel } from "../lib/format";
import { resolveShortcut, shortcutHint } from "../lib/keyboard";
import { redo, undo } from "../lib/tauri-api";
import { usePlanStore } from "../store/plan-store";
import { useUiStore } from "../store/ui-store";
import { ErrorDialog } from "./ErrorDialog";
import { MergeProgressLine } from "./MergeProgressLine";
import { ToolbarIconButton } from "./ToolbarIconButton";
import { useMerge } from "./useMerge";
import { ViewToggle } from "./ViewToggle";

async function showInFolder(dest: string): Promise<void> {
  try {
    await revealItemInDir(dest);
  } catch (error) {
    console.error("reveal item failed", error);
  }
}

/** The output's own name, which is all the completion chip has room for. */
function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
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
    <div className="relative flex h-12 items-center border-b border-slate-800 bg-slate-900/80 px-3.5 text-xs text-slate-400">
      <span>
        {countLabel(fileCount, "file")}
        <span aria-hidden="true" className="px-1.5 text-slate-600">
          ·
        </span>
        {countLabel(pageCount, "page")}
      </span>

      {/*
        The tools hold the window midpoint rather than following the left-hand
        readout, so they stay put as the counts and the merge result change
        width around them.
      */}
      <div className="absolute left-1/2 flex -translate-x-1/2 items-center gap-1.5">
        <ToolbarIconButton
          disabled={!canUndo || isMerging}
          icon={Undo2}
          label="Undo"
          onClick={() => void performUndo()}
          shortcut={shortcutHint("undo", navigator.userAgent)}
        />
        <ToolbarIconButton
          disabled={!canRedo || isMerging}
          icon={Redo2}
          label="Redo"
          onClick={() => void performRedo()}
          shortcut={shortcutHint("redo", navigator.userAgent)}
        />
        <span aria-hidden="true" className="mx-1 h-5 w-px bg-slate-700" />
        <ViewToggle />
      </div>

      <div className="ml-auto flex items-center gap-2">
        {isMerging && (
          <span>
            {progress.done} / {progress.total}
          </span>
        )}
        {result && (
          <>
            <span className="flex items-center gap-1.5">
              <span aria-hidden="true" className="size-1.5 rounded-full bg-emerald-500" />
              {fileName(result.dest)}
            </span>
            <button
              className="flex h-7 items-center gap-1.5 rounded-md border border-slate-700 px-2.5 text-slate-300 hover:bg-slate-800 focus-visible:outline-2 focus-visible:outline-sky-500"
              onClick={() => void showInFolder(result.dest)}
              type="button"
            >
              <FolderOpen aria-hidden="true" size={15} strokeWidth={1.8} />
              Show in folder
            </button>
          </>
        )}
        {failure && (
          <span className="flex items-center gap-1.5 text-red-400">
            <span aria-hidden="true" className="size-1.5 rounded-full bg-red-400" />
            Merge failed
          </span>
        )}
        <button
          className="h-7 rounded-md bg-sky-600 px-3.5 font-semibold text-white hover:bg-sky-500 focus-visible:outline-2 focus-visible:outline-sky-400 disabled:cursor-not-allowed disabled:bg-slate-700 disabled:text-slate-400"
          disabled={pageCount === 0 || isMerging}
          onClick={() => void start()}
          type="button"
        >
          {isMerging ? "Merging…" : "Merge"}
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
