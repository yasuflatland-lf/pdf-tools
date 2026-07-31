import { DndContext } from "@dnd-kit/core";
import { rectSortingStrategy, SortableContext } from "@dnd-kit/sortable";
import { useRef } from "react";
import { useUiStore } from "../store/ui-store";
import { PageListRow } from "./PageListRow";
import { SortableCard } from "./SortableCard";
import { useCardFocus, useCardRows, usePageCards } from "./usePageCards";

const ROW_HEIGHT = 112;
const THUMBNAIL_WIDTH = 160;

export function PageList() {
  const selectedSlots = useUiStore((state) => state.selectedSlots);
  const scrollRef = useRef<HTMLDivElement>(null);
  const { cache, cards, handleDragEnd, sensors } = usePageCards();
  const { rows, scrollToIndex, totalSize } = useCardRows({
    cards,
    columnCount: 1,
    rowHeight: ROW_HEIGHT,
    scrollRef,
  });
  const focusedIndex = useCardFocus({ cards, columnCount: 1, scrollToIndex });

  return (
    <DndContext sensors={sensors} onDragEnd={handleDragEnd}>
      <div
        ref={scrollRef}
        className="h-full overflow-y-auto"
        aria-label="Document pages"
        data-view-mode="list"
        role="listbox"
        aria-multiselectable="true"
      >
        <SortableContext items={cards.map((card) => card.key)} strategy={rectSortingStrategy}>
          <div className="relative w-full" style={{ height: `${totalSize}px` }}>
            {rows.map((row) => {
              const card = cards[row.index];
              const selected = card.slotIds.every((slotId) => selectedSlots.has(slotId));

              return (
                <div
                  key={row.key}
                  className="absolute top-0 left-0 w-full pb-4"
                  style={{ transform: `translateY(${row.start}px)` }}
                >
                  <SortableCard
                    id={card.key}
                    label={card.fileName}
                    focused={focusedIndex === row.index}
                    selected={selected}
                  >
                    <PageListRow
                      cache={cache}
                      collapsed={card.collapsed}
                      fileName={card.fileName}
                      onToggle={
                        card.collapsible
                          ? () => useUiStore.getState().toggleExpanded(card.slot.source)
                          : undefined
                      }
                      pageCount={card.pageCount}
                      pageNumber={card.slot.page + 1}
                      slotId={card.slot.id}
                      thumbnailWidth={THUMBNAIL_WIDTH}
                    />
                  </SortableCard>
                </div>
              );
            })}
          </div>
        </SortableContext>
      </div>
    </DndContext>
  );
}
