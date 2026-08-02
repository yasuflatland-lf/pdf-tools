import { ask, open } from "@tauri-apps/plugin-dialog";
import { useCallback } from "react";
import { errorMessage } from "../lib/error-message";
import { baseName } from "../lib/output-dir";
import { addSources, expandPaths, supportedExtensions } from "../lib/tauri-api";
import { usePlanStore } from "../store/plan-store";
import { useUiStore } from "../store/ui-store";

/**
 * Above this many files, one confirmation is worth the interruption. The
 * figure is twice the 100-page scale the merge target names: enough material
 * that a mis-picked folder costs more than the question does.
 */
const CONFIRM_THRESHOLD = 200;

interface AddSourcesController {
  isIngesting: boolean;
  chooseFiles: () => Promise<void>;
  chooseFolder: () => Promise<void>;
  addPaths: (paths: string[]) => Promise<void>;
}

/**
 * Runs the whole add procedure: expand, report an empty result, confirm a
 * large one, then hand the files to the plan.
 *
 * `label` names the single folder or file selected, or is `null` when there is
 * no single selection to name. An expansion returns nothing when no supported
 * files were found.
 */
async function ingest(paths: string[], label: string | null): Promise<void> {
  if (paths.length === 0) {
    return;
  }

  const ui = useUiStore.getState();
  if (ui.isIngesting) {
    return;
  }

  ui.setSourceNotice(null);
  ui.setIngesting(true);
  try {
    const expanded = await expandPaths(paths);

    if (expanded.length === 0) {
      ui.setSourceNotice(
        label === null
          ? "No PDFs or images found in the selection."
          : `No PDFs or images found in "${label}".`,
      );
      return;
    }

    if (expanded.length > CONFIRM_THRESHOLD) {
      const question =
        label === null
          ? `Add ${expanded.length} files?`
          : `Add ${expanded.length} files from "${label}"?`;
      if (!(await ask(question, { title: "Add files", kind: "info" }))) {
        return;
      }
    }

    usePlanStore.getState().setSnapshot(await addSources(expanded));
  } catch (error) {
    console.error("add sources failed", error);
    ui.setSourceNotice(errorMessage(error));
  } finally {
    ui.setIngesting(false);
  }
}

/**
 * The one way into the document. The toolbar menu and the drop listener both
 * come through here, so a folder cannot behave one way when it is dropped and
 * another when it is picked.
 */
export function useAddSources(): AddSourcesController {
  const isIngesting = useUiStore((state) => state.isIngesting);

  // Every callback reads its state through `getState`, so none of them close
  // over a render. Keeping them stable lets the drop listener register once.
  const chooseFiles = useCallback(async () => {
    const picked = await open({
      multiple: true,
      filters: [{ name: "PDFs and images", extensions: await supportedExtensions() }],
    });
    if (picked === null) {
      return;
    }

    const paths = Array.isArray(picked) ? picked : [picked];
    await ingest(paths, null);
  }, []);

  const chooseFolder = useCallback(async () => {
    const picked = await open({ directory: true });
    if (picked === null || Array.isArray(picked)) {
      return;
    }

    await ingest([picked], baseName(picked));
  }, []);

  const addPaths = useCallback(async (paths: string[]) => {
    await ingest(paths, paths.length === 1 ? baseName(paths[0]) : null);
  }, []);

  return { isIngesting, chooseFiles, chooseFolder, addPaths };
}
