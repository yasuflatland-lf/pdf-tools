import { invoke } from "@tauri-apps/api/core";
import type { PlanSnapshot } from "../bindings/PlanSnapshot";

export async function addSources(paths: string[]): Promise<PlanSnapshot> {
  return invoke<PlanSnapshot>("add_sources", { paths });
}
