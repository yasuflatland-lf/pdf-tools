import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ComposeProgressDto } from "../bindings/ComposeProgressDto";
import type { MergeReportDto } from "../bindings/MergeReportDto";
import type { PlanSnapshot } from "../bindings/PlanSnapshot";

export async function addSources(paths: string[]): Promise<PlanSnapshot> {
  return invoke<PlanSnapshot>("add_sources", { paths });
}

export async function rasterizeSlot(slotId: number, width: number): Promise<ArrayBuffer> {
  return invoke<ArrayBuffer>("rasterize_slot", { slotId, width });
}

export async function compose(dest: string): Promise<MergeReportDto> {
  return invoke<MergeReportDto>("compose", { dest });
}

export async function onComposeProgress(
  handler: (progress: ComposeProgressDto) => void,
): Promise<UnlistenFn> {
  return listen<ComposeProgressDto>("compose-progress", (event) => handler(event.payload));
}
