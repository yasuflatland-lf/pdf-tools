import { create } from "zustand";

/**
 * View state that only the frontend owns. Nothing here is part of the document,
 * so it survives snapshot replacement untouched.
 */
interface UiState {
  expandedSources: Set<number>;
  selectedSlots: Set<number>;
  viewMode: "grid" | "list";
  toggleExpanded: (sourceId: number) => void;
  pruneExpanded: (sourceIds: number[]) => void;
}

export function createUiStore() {
  return create<UiState>((set, get) => ({
    expandedSources: new Set(),
    selectedSlots: new Set(),
    viewMode: "grid",
    toggleExpanded: (sourceId) => {
      const expandedSources = new Set(get().expandedSources);
      if (expandedSources.has(sourceId)) {
        expandedSources.delete(sourceId);
      } else {
        expandedSources.add(sourceId);
      }
      set({ expandedSources });
    },
    pruneExpanded: (sourceIds) => {
      const current = get().expandedSources;
      const existing = new Set(sourceIds);
      if ([...current].every((sourceId) => existing.has(sourceId))) {
        return;
      }

      set({
        expandedSources: new Set([...current].filter((sourceId) => existing.has(sourceId))),
      });
    },
  }));
}

export const useUiStore = createUiStore();
