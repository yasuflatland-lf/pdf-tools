import { create } from "zustand";

/**
 * View state that only the frontend owns. Nothing here is part of the document,
 * so it survives snapshot replacement untouched.
 */
interface UiState {
  expandedSources: Set<number>;
  selectedSlots: Set<number>;
  viewMode: "grid" | "list";
}

export const useUiStore = create<UiState>(() => ({
  expandedSources: new Set(),
  selectedSlots: new Set(),
  viewMode: "grid",
}));
