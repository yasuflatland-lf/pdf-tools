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
  /**
   * Whether a modal owns the keyboard. The shared document shortcut listener
   * checks this before dispatching any action behind the overlay.
   */
  modalOpen: boolean;
  setModalOpen: (open: boolean) => void;
  setViewMode: (mode: ViewMode) => void;
  toggleExpanded: (sourceId: number) => void;
  pruneExpanded: (sourceIds: number[]) => void;
  selectSlots: (slotIds: number[]) => void;
  clearSelection: () => void;
  pruneSelected: (slotIds: number[]) => void;
}

export function createUiStore() {
  return create<UiState>((set, get) => ({
    expandedSources: new Set(),
    selectedSlots: new Set(),
    viewMode: loadPersistedViewMode(),
    modalOpen: false,
    setModalOpen: (modalOpen) => {
      set({ modalOpen });
    },
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
    selectSlots: (slotIds) => {
      set({ selectedSlots: new Set(slotIds) });
    },
    // An already empty selection keeps its set, so Escape on nothing cannot
    // re-render every card that subscribes to it.
    clearSelection: () => {
      if (get().selectedSlots.size > 0) {
        set({ selectedSlots: new Set() });
      }
    },
    pruneSelected: (slotIds) => {
      const current = get().selectedSlots;
      const existing = new Set(slotIds);
      if ([...current].every((slotId) => existing.has(slotId))) {
        return;
      }

      set({ selectedSlots: new Set([...current].filter((slotId) => existing.has(slotId))) });
    },
  }));
}

export const useUiStore = createUiStore();
