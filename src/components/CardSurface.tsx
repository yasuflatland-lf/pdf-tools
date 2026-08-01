import { DndContext } from "@dnd-kit/core";
import { rectSortingStrategy, SortableContext } from "@dnd-kit/sortable";
import { useRef, type ReactNode, type RefObject } from "react";
import { useUiStore } from "../store/ui-store";
import { SortableCard } from "./SortableCard";
import { useCardFocus, useCardRows, usePageCards, type DisplayCard } from "./usePageCards";

interface CardSurfaceProps {
  columnCount: number;
  rowHeight: number;
  viewMode: "grid" | "list";
  renderCard: (card: DisplayCard, thumbnailWidth: number, selected: boolean) => ReactNode;
  scrollRef?: RefObject<HTMLDivElement | null>;
  thumbnailWidth: number;
}

export function CardSurface({
  columnCount,
  rowHeight,
  viewMode,
  renderCard,
  scrollRef,
  thumbnailWidth,
}: CardSurfaceProps) {
  const selectedSlots = useUiStore((state) => state.selectedSlots);
  const ownScrollRef = useRef<HTMLDivElement>(null);
  const containerRef = scrollRef ?? ownScrollRef;
  const { cards, handleDragEnd, sensors } = usePageCards();
  const { rows, scrollToIndex, totalSize } = useCardRows({
    cards,
    columnCount,
    rowHeight,
    scrollRef: containerRef,
  });
  const { focusedIndex, selectCard } = useCardFocus({ cards, columnCount, scrollToIndex });

  return (
    <DndContext sensors={sensors} onDragEnd={handleDragEnd}>
      <div
        ref={containerRef}
        className="h-full overflow-y-auto"
        aria-label="Document pages"
        data-view-mode={viewMode}
        role="listbox"
        aria-multiselectable="true"
      >
        <SortableContext items={cards.map((card) => card.key)} strategy={rectSortingStrategy}>
          <div className="relative w-full" style={{ height: `${totalSize}px` }}>
            {rows.map((row) => {
              const rowCards = cards.slice(row.index * columnCount, (row.index + 1) * columnCount);

              return (
                <div
                  key={row.key}
                  className="absolute top-0 left-0 grid w-full gap-4 pb-4"
                  style={{
                    gridTemplateColumns: `repeat(${columnCount}, minmax(0, 1fr))`,
                    transform: `translateY(${row.start}px)`,
                  }}
                >
                  {rowCards.map((card, positionInRow) => {
                    const cardIndex = row.index * columnCount + positionInRow;
                    const selected = card.slotIds.every((slotId) => selectedSlots.has(slotId));

                    return (
                      <SortableCard
                        key={card.key}
                        id={card.key}
                        label={card.fileName}
                        rotation={card.slot.rotation}
                        focused={focusedIndex === cardIndex}
                        onSelect={(modifiers) => selectCard(cardIndex, modifiers)}
                        selected={selected}
                      >
                        {renderCard(card, thumbnailWidth, selected)}
                      </SortableCard>
                    );
                  })}
                </div>
              );
            })}
          </div>
        </SortableContext>
      </div>
    </DndContext>
  );
}
