/**
 * Formats a count with its noun, pluralised the English way. Shared so the
 * toolbar summary and the source list never drift apart.
 */
export function countLabel(count: number, singular: string): string {
  return `${count} ${singular}${count === 1 ? "" : "s"}`;
}
