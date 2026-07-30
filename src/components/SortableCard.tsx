import { useSortable } from "@dnd-kit/sortable";
import type { ReactNode } from "react";

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
  id: string;
  label: string;
}

export function SortableCard({ children, id, label }: SortableCardProps) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id,
  });

  return (
    <div
      ref={setNodeRef}
      className="relative"
      style={{
        transform: transformToString(transform),
        transition,
        opacity: isDragging ? 0.4 : 1,
      }}
      {...attributes}
      {...listeners}
      aria-label={`Reorder ${label}`}
    >
      {children}
    </div>
  );
}
