import { useEffect } from "react";
import { usePlanStore } from "./plan-store";
import { useUiStore } from "./ui-store";

/**
 * Keeps view state consistent with the document. Expansion belongs to a source
 * and outlives any operation that only ungroups it: turning one page of a run,
 * or dragging through it, must not cost the user the expansion when the run
 * refolds. A selection, in contrast, may not point at slots the document no
 * longer has. Mounted once, by AppShell.
 */
export function useSnapshotSync(): void {
  useEffect(
    () =>
      usePlanStore.subscribe((state) => {
        const ui = useUiStore.getState();
        ui.pruneExpanded(state.sources.map((source) => source.id));
        ui.pruneSelected(state.slots.map((slot) => slot.id));
      }),
    [],
  );
}
