/**
 * Shortcut resolution lives here as pure functions rather than inside the
 * components that listen for the key: the mapping is where the mistakes are, and
 * it is the part worth testing without a DOM, a store or an IPC round trip.
 */
export type ShortcutAction =
  | "remove-selected"
  | "select-all"
  | "clear-selection"
  | "undo"
  | "redo"
  | "rotate-left"
  | "rotate-right"
  | "focus-previous"
  | "focus-next"
  | "focus-row-previous"
  | "focus-row-next";

/** The subset of `KeyboardEvent` a shortcut is resolved from. */
export interface ShortcutEvent {
  key: string;
  metaKey?: boolean;
  ctrlKey?: boolean;
  shiftKey?: boolean;
  altKey?: boolean;
  target?: EventTarget | null;
}

/**
 * Duck-typed rather than checked with `instanceof` so the resolver never needs a
 * DOM, and so it still recognises an input rendered inside another realm.
 */
function isTextEntry(target: EventTarget | null | undefined): boolean {
  if (typeof target !== "object" || target === null) {
    return false;
  }

  const candidate = target as { tagName?: unknown; isContentEditable?: unknown };
  const tagName =
    typeof candidate.tagName === "string" ? candidate.tagName.toUpperCase() : undefined;
  return (
    tagName === "INPUT" ||
    tagName === "TEXTAREA" ||
    tagName === "SELECT" ||
    candidate.isContentEditable === true
  );
}

/**
 * Maps one key press to an action, or to null when the window should ignore it.
 * A text entry always wins: a Delete typed into a filename field must edit the
 * text, never the document.
 */
export function resolveShortcut(event: ShortcutEvent): ShortcutAction | null {
  if (isTextEntry(event.target) || event.altKey === true) {
    return null;
  }

  const modifier = event.metaKey === true || event.ctrlKey === true;
  const key = event.key.toLowerCase();

  if (modifier) {
    if (key === "a") {
      return "select-all";
    }
    if (key === "z") {
      return event.shiftKey === true ? "redo" : "undo";
    }
    if (key === "[") {
      return "rotate-left";
    }
    if (key === "]") {
      return "rotate-right";
    }
    return null;
  }

  switch (event.key) {
    case "Delete":
    case "Backspace":
      return "remove-selected";
    case "Escape":
      return "clear-selection";
    case "ArrowLeft":
      return "focus-previous";
    case "ArrowRight":
      return "focus-next";
    case "ArrowUp":
      return "focus-row-previous";
    case "ArrowDown":
      return "focus-row-next";
    default:
      return null;
  }
}

/** The actions whose shortcut is worth printing in a tooltip. */
type HintableAction = Extract<ShortcutAction, "undo" | "redo">;

/**
 * How a shortcut is spelled for the reader. The user agent is a parameter
 * rather than a global so the mapping stays testable without a DOM, like the
 * rest of this file.
 */
export function shortcutHint(action: HintableAction, userAgent: string): string {
  const isMac = /mac|iphone|ipad/i.test(userAgent);
  if (isMac) {
    return action === "redo" ? "⇧⌘Z" : "⌘Z";
  }
  return action === "redo" ? "Ctrl+Shift+Z" : "Ctrl+Z";
}

/**
 * Where an arrow key moves the card focus, clamped to the grid. Returns null for
 * every action that is not a focus move, so a caller can hand it whatever
 * `resolveShortcut` produced. A `currentIndex` outside the grid means nothing is
 * focused yet, and the first or last card is entered instead.
 */
export function nextFocusIndex(
  action: ShortcutAction,
  currentIndex: number,
  count: number,
  columns: number,
): number | null {
  const columnCount = Number.isFinite(columns) ? Math.max(1, Math.floor(columns)) : 1;
  const steps: Partial<Record<ShortcutAction, number>> = {
    "focus-previous": -1,
    "focus-next": 1,
    "focus-row-previous": -columnCount,
    "focus-row-next": columnCount,
  };
  const step = steps[action];

  if (step === undefined || count <= 0) {
    return null;
  }
  if (currentIndex < 0 || currentIndex >= count) {
    return step > 0 ? 0 : count - 1;
  }

  return Math.min(count - 1, Math.max(0, currentIndex + step));
}
