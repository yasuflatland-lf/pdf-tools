import { create } from "zustand";

const VIEW_MODE_KEY = "pdf-tools.view-mode";

export type ViewMode = "grid" | "list";

export function loadPersistedViewMode(): ViewMode {
  try {
    const stored = localStorage.getItem(VIEW_MODE_KEY);
    return stored === "grid" || stored === "list" ? stored : "grid";
  } catch {
    return "grid";
  }
}

/**
 * View state that only the frontend owns. Nothing here is part of the document,
 * so it survives snapshot replacement untouched.
 */
interface UiState {
  expandedSources: Set<number>;
  selectedSlots: Set<number>;
  viewMode: ViewMode;
  setViewMode: (mode: ViewMode) => void;
  toggleExpanded: (sourceId: number) => void;
  pruneExpanded: (sourceIds: number[]) => void;
}

export function createUiStore() {
  return create<UiState>((set, get) => ({
    expandedSources: new Set(),
    selectedSlots: new Set(),
    viewMode: loadPersistedViewMode(),
    setViewMode: (viewMode) => {
      set({ viewMode });
      try {
        localStorage.setItem(VIEW_MODE_KEY, viewMode);
      } catch {
        // The in-memory preference remains usable when storage is unavailable.
      }
    },
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
