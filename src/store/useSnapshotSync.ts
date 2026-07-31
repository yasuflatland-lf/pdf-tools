import { useEffect } from "react";
import { usePlanStore } from "./plan-store";
import { useUiStore } from "./ui-store";

/**
 * Keeps view state consistent with the document. Expansion only means something
 * for a source that is still present and still grouped; a selection may not
 * point at slots the document no longer has. Mounted once, by AppShell.
 */
export function useSnapshotSync(): void {
  useEffect(
    () =>
      usePlanStore.subscribe((state) => {
        const ui = useUiStore.getState();
        ui.pruneExpanded(
          state.sources
            .filter((source) => source.grouping === "grouped")
            .map((source) => source.id),
        );
        ui.pruneSelected(state.slots.map((slot) => slot.id));
      }),
    [],
  );
}
