import { save } from "@tauri-apps/plugin-dialog";
import { useCallback, useState } from "react";
import type { ComposeProgressDto } from "../bindings/ComposeProgressDto";
import type { MergeReportDto } from "../bindings/MergeReportDto";
import { errorMessage } from "../lib/error-message";
import { defaultOutputDir, joinPath, parentDir, rememberOutputDir } from "../lib/output-dir";
import { compose, onComposeProgress } from "../lib/tauri-api";
import { usePlanStore } from "../store/plan-store";
import { useUiStore } from "../store/ui-store";
import { blamedFiles } from "./ErrorDialog";

interface MergeResult {
  dest: string;
  report: MergeReportDto;
}

interface MergeFailure {
  files: string[];
  message: string;
}

interface MergeController {
  isMerging: boolean;
  progress: ComposeProgressDto;
  result: MergeResult | null;
  failure: MergeFailure | null;
  start: () => Promise<void>;
}

/**
 * The whole merge procedure -- ask for a destination, subscribe to progress,
 * compose, and record what happened. It lives apart from the toolbar because
 * the toolbar's own job is placing things in a row.
 */
export function useMerge(): MergeController {
  const [isMerging, setIsMerging] = useState(false);
  const [progress, setProgress] = useState<ComposeProgressDto>({ done: 0, total: 0 });
  const [result, setResult] = useState<MergeResult | null>(null);
  // Dismissing the dialog must not erase the failure itself: the toolbar keeps
  // its marker until the next merge, so a merge that produced nothing is never
  // mistaken for one that has not been run.
  const [failure, setFailure] = useState<MergeFailure | null>(null);

  const start = useCallback(async () => {
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
      useUiStore.getState().setModalOpen(false);

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
      useUiStore.getState().setModalOpen(true);
    } finally {
      setIsMerging(false);
    }
  }, []);

  return { isMerging, progress, result, failure, start };
}
