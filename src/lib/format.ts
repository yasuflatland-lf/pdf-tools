import type { SourceStatusDto } from "../bindings/SourceStatusDto";

/**
 * Formats a count with its noun, pluralised the English way. Shared so the
 * toolbar summary and the source list never drift apart.
 */
export function countLabel(count: number, singular: string): string {
  return `${count} ${singular}${count === 1 ? "" : "s"}`;
}

/**
 * Names a source file's state in one short phrase. Shared so the unusable-file
 * list and the list view report the same wording for the same status.
 */
export function statusLabel(status: SourceStatusDto): string {
  if (status.kind === "ready") {
    return "Ready";
  }
  if (status.kind === "encrypted") {
    return "Encrypted";
  }
  return `Unreadable: ${status.reason}`;
}
