import type { PageSlotDto } from "../bindings/PageSlotDto";
import type { SourceFileDto } from "../bindings/SourceFileDto";

/**
 * One card in the page grid. A group holds every slot it collapses, so the grid
 * can expand it into per-page cards without recomputing the run, and reports
 * `pageCount` from the slots actually present -- not from `source.page_count`,
 * which still counts pages the user has since deleted.
 */
export interface DisplayGroup {
  key: string;
  sourceId: number;
  slots: PageSlotDto[];
  slotIds: number[];
  pageCount: number;
}

function createGroup(slot: PageSlotDto): DisplayGroup {
  return {
    key: `source-${slot.source}-slot-${slot.id}`,
    sourceId: slot.source,
    slots: [slot],
    slotIds: [slot.id],
    pageCount: 1,
  };
}

/**
 * Mirrors `domain::grouping::group_contiguous`: a run of neighbouring slots from
 * one source collapses into a single card, but only while that source is
 * grouped. Ungrouped and unknown sources yield one card per page.
 */
export function groupContiguous(slots: PageSlotDto[], sources: SourceFileDto[]): DisplayGroup[] {
  const sourcesById = new Map(sources.map((source) => [source.id, source]));
  const groups: DisplayGroup[] = [];

  for (const slot of slots) {
    const source = sourcesById.get(slot.source);
    const previous = groups.at(-1);

    if (!previous || source?.grouping !== "grouped" || previous.sourceId !== slot.source) {
      groups.push(createGroup(slot));
      continue;
    }

    previous.slots.push(slot);
    previous.slotIds.push(slot.id);
    previous.pageCount += 1;
  }

  return groups;
}
