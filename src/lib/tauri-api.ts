import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ComposeProgressDto } from "../bindings/ComposeProgressDto";
import type { MergeReportDto } from "../bindings/MergeReportDto";
import type { PlanSnapshot } from "../bindings/PlanSnapshot";

export async function addSources(paths: string[]): Promise<PlanSnapshot> {
  return invoke<PlanSnapshot>("add_sources", { paths });
}

/**
 * Resolves folders to the files inside them and passes everything else
 * through. Separate from `addSources` so the count can be seen, and asked
 * about, before anything enters the plan.
 */
export async function expandPaths(paths: string[]): Promise<string[]> {
  return invoke<string[]>("expand_paths", { paths });
}

export async function rasterizeSlot(slotId: number, width: number): Promise<ArrayBuffer> {
  return invoke<ArrayBuffer>("rasterize_slot", { slotId, width });
}

export async function reorder(
  fromStart: number,
  fromEnd: number,
  to: number,
): Promise<PlanSnapshot> {
  return invoke<PlanSnapshot>("reorder", { fromStart, fromEnd, to });
}

export async function removeSlots(slotIds: number[]): Promise<PlanSnapshot> {
  return invoke<PlanSnapshot>("remove_slots", { slotIds });
}

export async function removeSource(sourceId: number): Promise<PlanSnapshot> {
  return invoke<PlanSnapshot>("remove_source", { sourceId });
}

export async function rotateSlots(slotIds: number[], delta: number): Promise<PlanSnapshot> {
  return invoke<PlanSnapshot>("rotate_slots", { slotIds, delta });
}

export async function undo(): Promise<PlanSnapshot> {
  return invoke<PlanSnapshot>("undo");
}

export async function redo(): Promise<PlanSnapshot> {
  return invoke<PlanSnapshot>("redo");
}

export async function compose(dest: string): Promise<MergeReportDto> {
  return invoke<MergeReportDto>("compose", { dest });
}

export async function onComposeProgress(
  handler: (progress: ComposeProgressDto) => void,
): Promise<UnlistenFn> {
  return listen<ComposeProgressDto>("compose-progress", (event) => handler(event.payload));
}
