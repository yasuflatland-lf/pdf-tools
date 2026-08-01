import { useSortable } from "@dnd-kit/sortable";
import { useCallback, useEffect, useRef, type ReactNode } from "react";

interface Transform {
  x: number;
  y: number;
  scaleX: number;
  scaleY: number;
}

/**
 * Equivalent to `CSS.Transform.toString` from `@dnd-kit/utilities`, which is
 * only a transitive dependency here. Spelling the transform out keeps the
 * package a private implementation detail of dnd-kit rather than something this
 * project imports without declaring.
 */
function transformToString(transform: Transform | null): string | undefined {
  if (!transform) {
    return undefined;
  }

  return `translate3d(${Math.round(transform.x)}px, ${Math.round(transform.y)}px, 0) scaleX(${transform.scaleX}) scaleY(${transform.scaleY})`;
}

interface SortableCardProps {
  children: ReactNode;
  focused?: boolean;
  id: string;
  label: string;
  rotation: number;
  selected?: boolean;
}

export function SortableCard({
  children,
  focused,
  id,
  label,
  rotation,
  selected,
}: SortableCardProps) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id,
    // The drag wrapper is the element the grid's listbox owns, so the option
    // role and the selection state belong here. On a node further in they would
    // sit behind dnd-kit's own `role="button"`, and `aria-selected` outside a
    // listbox is announced by nothing.
    attributes: { role: "option" },
  });
  const cardRef = useRef<HTMLDivElement | null>(null);
  // Memoized so the node is not detached from and reattached to dnd-kit on
  // every render, which a drag produces plenty of.
  const setCardRef = useCallback(
    (node: HTMLDivElement | null) => {
      setNodeRef(node);
      cardRef.current = node;
    },
    [setNodeRef],
  );

  // Arrow navigation only moves an index; the card the index lands on is what
  // takes the DOM focus. A card scrolled into view by the virtualizer mounts
  // already focused, so this also covers the rows that were not rendered yet.
  useEffect(() => {
    if (focused) {
      cardRef.current?.focus();
    }
  }, [focused]);

  return (
    <div
      ref={setCardRef}
      className="relative"
      style={{
        transform: transformToString(transform),
        transition,
        opacity: isDragging ? 0.4 : 1,
      }}
      {...attributes}
      {...listeners}
      aria-label={`Reorder ${label}${rotation !== 0 ? `, rotation ${rotation * 90}° clockwise` : ""}`}
      aria-selected={selected === true}
    >
      {children}
    </div>
  );
}
