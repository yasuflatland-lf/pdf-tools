import { invoke } from "@tauri-apps/api/core";
import type { PlanSnapshot } from "../bindings/PlanSnapshot";

export async function addSources(paths: string[]): Promise<PlanSnapshot> {
  return invoke<PlanSnapshot>("add_sources", { paths });
}

export async function rasterizeSlot(slotId: number, width: number): Promise<ArrayBuffer> {
  return invoke<ArrayBuffer>("rasterize_slot", { slotId, width });
}
