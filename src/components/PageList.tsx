import { DndContext } from "@dnd-kit/core";
import { rectSortingStrategy, SortableContext } from "@dnd-kit/sortable";
import { useRef } from "react";
import { useUiStore } from "../store/ui-store";
import { PageListRow } from "./PageListRow";
import { SortableCard } from "./SortableCard";
import { useCardRows, usePageCards } from "./usePageCards";

const ROW_HEIGHT = 112;
const THUMBNAIL_WIDTH = 160;

export function PageList() {
  const scrollRef = useRef<HTMLDivElement>(null);
  const { cache, cards, handleDragEnd, sensors } = usePageCards();
  const { rows, totalSize } = useCardRows({
    cards,
    columnCount: 1,
    rowHeight: ROW_HEIGHT,
    scrollRef,
  });

  return (
    <DndContext sensors={sensors} onDragEnd={handleDragEnd}>
      <div
        ref={scrollRef}
        className="h-full overflow-y-auto"
        aria-label="Document pages"
        data-view-mode="list"
      >
        <SortableContext items={cards.map((card) => card.key)} strategy={rectSortingStrategy}>
          <div className="relative w-full" style={{ height: `${totalSize}px` }}>
            {rows.map((row) => {
              const card = cards[row.index];

              return (
                <div
                  key={row.key}
                  className="absolute top-0 left-0 w-full pb-4"
                  style={{ transform: `translateY(${row.start}px)` }}
                >
                  <SortableCard id={card.key} label={card.fileName}>
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
