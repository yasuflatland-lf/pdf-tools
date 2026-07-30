import { save } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useCallback, useEffect, useState } from "react";
import type { ComposeProgressDto } from "../bindings/ComposeProgressDto";
import type { MergeReportDto } from "../bindings/MergeReportDto";
import { countLabel } from "../lib/format";
import { resolveShortcut } from "../lib/keyboard";
import { defaultOutputDir, joinPath, parentDir, rememberOutputDir } from "../lib/output-dir";
import { compose, onComposeProgress, redo, undo } from "../lib/tauri-api";
import { usePlanStore } from "../store/plan-store";
import { useUiStore } from "../store/ui-store";
import { blamedFiles, ErrorDialog } from "./ErrorDialog";
import { ProgressBar } from "./ProgressBar";

interface MergeResult {
  dest: string;
  report: MergeReportDto;
}

interface MergeFailure {
  files: string[];
  message: string;
}

function errorMessage(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

async function showInFolder(dest: string): Promise<void> {
  try {
    await revealItemInDir(dest);
  } catch (error) {
    console.error("reveal item failed", error);
  }
}

/**
 * The active view button is filled in, so the current mode reads at a glance
 * rather than only through `aria-pressed`.
 */
function viewButtonClass(active: boolean): string {
  const state = active
    ? "border-slate-500 bg-slate-800 text-slate-100"
    : "border-slate-700 hover:bg-slate-800";
  return `rounded-md border px-3 py-1.5 ${state}`;
}

export function Toolbar() {
  const fileCount = usePlanStore((state) => state.sources.length);
  const pageCount = usePlanStore((state) => state.slots.length);
  const canUndo = usePlanStore((state) => state.canUndo);
  const canRedo = usePlanStore((state) => state.canRedo);
  const viewMode = useUiStore((state) => state.viewMode);
  const setViewMode = useUiStore((state) => state.setViewMode);
  const [isMerging, setIsMerging] = useState(false);
  const [progress, setProgress] = useState<ComposeProgressDto>({ done: 0, total: 0 });
  const [result, setResult] = useState<MergeResult | null>(null);
  const [failure, setFailure] = useState<MergeFailure | null>(null);
  // Dismissing the dialog must not erase the failure itself: the toolbar keeps
  // its marker until the next merge, so a merge that produced nothing is never
  // mistaken for one that has not been run.
  const [isDialogOpen, setIsDialogOpen] = useState(false);

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

  async function merge(): Promise<void> {
    try {
      const dir = await defaultOutputDir();
      const dest = await save({
        defaultPath: joinPath(dir, "merged.pdf"),
        filters: [{ name: "PDF", extensions: ["pdf"] }],
      });
      if (!dest) {
        return;
      }

      rememberOutputDir(parentDir(dest));
      setIsMerging(true);
      setProgress({ done: 0, total: 0 });
      setResult(null);
      setFailure(null);
      setIsDialogOpen(false);

      let unlisten: (() => void) | undefined;
      try {
        unlisten = await onComposeProgress(setProgress);
        const report = await compose(dest);
        setResult({ dest, report });
      } finally {
        unlisten?.();
      }
    } catch (error) {
      console.error("compose failed", error);
      const message = errorMessage(error);
      setFailure({
        files: blamedFiles(message, usePlanStore.getState().sources),
        message,
      });
      setIsDialogOpen(true);
    } finally {
      setIsMerging(false);
    }
  }

  return (
    <div className="flex items-center gap-3 border-y border-slate-800 bg-slate-900/80 px-6 py-3 text-sm text-slate-300">
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
      <div className="flex items-center gap-2" role="group" aria-label="View mode">
        <button
          aria-pressed={viewMode === "grid"}
          className={viewButtonClass(viewMode === "grid")}
          onClick={() => setViewMode("grid")}
          type="button"
        >
          Grid
        </button>
        <button
          aria-pressed={viewMode === "list"}
          className={viewButtonClass(viewMode === "list")}
          onClick={() => setViewMode("list")}
          type="button"
        >
          List
        </button>
      </div>
      <div className="ml-auto flex items-center gap-3">
        {isMerging && (
          <ProgressBar done={progress.done} label="Merge progress" total={progress.total} />
        )}
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
          onClick={() => void merge()}
          type="button"
        >
          Merge
        </button>
      </div>
      {failure && isDialogOpen && (
        <ErrorDialog
          files={failure.files}
          message={failure.message}
          onClose={() => setIsDialogOpen(false)}
        />
      )}
    </div>
  );
}
