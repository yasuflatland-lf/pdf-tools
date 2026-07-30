import { save } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useState } from "react";
import type { ComposeProgressDto } from "../bindings/ComposeProgressDto";
import type { MergeReportDto } from "../bindings/MergeReportDto";
import { countLabel } from "../lib/format";
import { defaultOutputDir, joinPath, parentDir, rememberOutputDir } from "../lib/output-dir";
import { compose, onComposeProgress } from "../lib/tauri-api";
import { usePlanStore } from "../store/plan-store";
import { ProgressBar } from "./ProgressBar";

interface MergeResult {
  dest: string;
  report: MergeReportDto;
}

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
  const [isMerging, setIsMerging] = useState(false);
  const [progress, setProgress] = useState<ComposeProgressDto>({ done: 0, total: 0 });
  const [result, setResult] = useState<MergeResult | null>(null);
  const [failed, setFailed] = useState(false);

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
      setFailed(false);

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
      setFailed(true);
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
        {failed && <span className="text-red-400">Merge failed</span>}
        <button
          className="rounded-md bg-sky-600 px-4 py-1.5 font-medium text-white hover:bg-sky-500 disabled:cursor-not-allowed disabled:bg-slate-700 disabled:text-slate-400"
          disabled={pageCount === 0 || isMerging}
          onClick={() => void merge()}
          type="button"
        >
          Merge
        </button>
      </div>
    </div>
  );
}
