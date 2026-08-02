import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

/**
 * What every control drawn on a card shares. The layout is what differs
 * between a control carrying text and one carrying an icon, so it is the only
 * thing the two exports below add.
 */
const CHROME =
  "rounded-md border border-slate-600 bg-slate-900/90 text-slate-200 shadow hover:bg-slate-800 focus-visible:ring-2 focus-visible:ring-sky-400 focus-visible:outline-none";

interface CardControlProps {
  /** The accessible name. It names the card as well as the action, because a
   *  screen reader reaching this control has no other way to tell the cards apart. */
  label: string;
  onPress: () => void;
  /** Positioning from the caller, which is what knows where on the card this sits. */
  className?: string;
}

/**
 * Both handlers are stopped for one reason: this button sits inside the node
 * the grid makes draggable and focusable, so an event left to bubble steers the
 * surface behind the control instead of pressing it. A keydown would reach the
 * shortcut listener; a pointerdown would start a drag.
 */
export function CardControl({
  label,
  onPress,
  className = "",
  children,
}: CardControlProps & { children: ReactNode }) {
  return (
    <button
      aria-label={label}
      className={`${CHROME} px-2 py-1 text-xs font-medium ${className}`}
      onClick={onPress}
      onKeyDown={(event) => event.stopPropagation()}
      onPointerDown={(event) => event.stopPropagation()}
      type="button"
    >
      {children}
    </button>
  );
}

/**
 * The same control with an icon in place of text. `title` is separate from
 * `label` because the tooltip names the action alone, while the accessible name
 * has to name the card too.
 */
export function CardIconControl({
  label,
  onPress,
  className = "",
  icon: Icon,
  title,
}: CardControlProps & { icon: LucideIcon; title: string }) {
  return (
    <button
      aria-label={label}
      className={`${CHROME} grid size-8 place-items-center ${className}`}
      onClick={onPress}
      onKeyDown={(event) => event.stopPropagation()}
      onPointerDown={(event) => event.stopPropagation()}
      title={title}
      type="button"
    >
      <Icon aria-hidden="true" size={16} strokeWidth={1.8} />
    </button>
  );
}
