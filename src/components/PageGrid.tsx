import { DndContext } from "@dnd-kit/core";
import { rectSortingStrategy, SortableContext } from "@dnd-kit/sortable";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { nextFocusIndex, resolveShortcut } from "../lib/keyboard";
import { useUiStore } from "../store/ui-store";
import { GroupCard } from "./GroupCard";
import { SortableCard } from "./SortableCard";
import { useCardRows, usePageCards } from "./usePageCards";

const CARD_MIN_WIDTH = 180;
const GAP = 16;
const ROW_HEIGHT = 332;
const THUMBNAIL_WIDTH = 360;

function getColumnCount(width: number): number {
  return Math.max(1, Math.floor((width + GAP) / (CARD_MIN_WIDTH + GAP)));
}

export function PageGrid() {
  const selectedSlots = useUiStore((state) => state.selectedSlots);
  const scrollRef = useRef<HTMLDivElement>(null);
  const [columnCount, setColumnCount] = useState(1);
  const [focusedIndex, setFocusedIndex] = useState<number | null>(null);
  const { cache, cards, handleDragEnd, sensors } = usePageCards();
  const { rows, scrollToIndex, totalSize } = useCardRows({
    cards,
    columnCount,
    rowHeight: ROW_HEIGHT,
    scrollRef,
  });

  useLayoutEffect(() => {
    const measure = () => {
      setColumnCount(getColumnCount(scrollRef.current?.clientWidth ?? 0));
    };

    measure();
    window.addEventListener("resize", measure);
    return () => window.removeEventListener("resize", measure);
  }, []);

  /**
   * Arrow navigation. It belongs to the grid because only the grid knows how
   * many cards there are and how many fit in a row, and the selection follows
   * the focus so that Delete always acts on the card the user is looking at.
   */
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      // dnd-kit's keyboard sensor steers a picked-up card with the same arrows
      // and calls `preventDefault` while it does; moving the focus as well would
      // drag one card and select another.
      if (event.defaultPrevented) {
        return;
      }

      // `nextFocusIndex` yields null for every action that is not a focus move,
      // which is how the other shortcuts stay the concern of their own handler.
      const action = resolveShortcut(event);
      const current = focusedIndex !== null && focusedIndex < cards.length ? focusedIndex : -1;
      const next =
        action === null ? null : nextFocusIndex(action, current, cards.length, columnCount);
      if (next === null) {
        return;
      }

      event.preventDefault();
      setFocusedIndex(next);
      scrollToIndex(Math.floor(next / columnCount));
      useUiStore.getState().selectSlots(cards[next].slotIds);
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [cards, columnCount, focusedIndex, scrollToIndex]);

  return (
    <DndContext sensors={sensors} onDragEnd={handleDragEnd}>
      <div
        ref={scrollRef}
        className="h-full overflow-y-auto"
        aria-label="Document pages"
        data-view-mode="grid"
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
                        focused={focusedIndex === cardIndex}
                        selected={selected}
                      >
                        <GroupCard
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
                          selected={selected}
                        />
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
