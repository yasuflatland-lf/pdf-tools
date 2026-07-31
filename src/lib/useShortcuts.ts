import { useEffect, useRef } from "react";
import type { ShortcutAction } from "./keyboard";
import { resolveShortcut } from "./keyboard";
import { usePlanStore } from "../store/plan-store";
import { useUiStore } from "../store/ui-store";

/** Returns true when the handler acted, which is what suppresses the default. */
type ShortcutHandler = (event: KeyboardEvent) => boolean;

export type ShortcutHandlers = Partial<Record<ShortcutAction, ShortcutHandler>>;

interface ShortcutRegistration {
  current: ShortcutHandlers;
}

const registrations = new Set<ShortcutRegistration>();

function handleKeyDown(event: KeyboardEvent): void {
  if (useUiStore.getState().modalOpen || event.defaultPrevented) {
    return;
  }

  const action = resolveShortcut(event);
  if (action === null) {
    return;
  }

  if (action === "rotate-left" || action === "rotate-right") {
    const selected = [...useUiStore.getState().selectedSlots];
    if (selected.length === 0) {
      return;
    }

    const delta = action === "rotate-right" ? 1 : -1;
    void usePlanStore
      .getState()
      .rotate(selected, delta)
      .catch((error: unknown) => console.error("rotate failed", error));
    event.preventDefault();
    return;
  }

  for (const registration of registrations) {
    const handler = registration.current[action];
    if (handler !== undefined) {
      if (handler(event)) {
        event.preventDefault();
      }
      return;
    }
  }
}

function register(registration: ShortcutRegistration): () => void {
  registrations.add(registration);
  if (registrations.size === 1) {
    window.addEventListener("keydown", handleKeyDown);
  }

  return () => {
    registrations.delete(registration);
    if (registrations.size === 0) {
      window.removeEventListener("keydown", handleKeyDown);
    }
  };
}

/**
 * The one window-level listener for document shortcuts. A modal owns the
 * keyboard while it is open, and anything already handled -- a card drag steers
 * with the same arrows -- is not a shortcut.
 */
export function useShortcuts(handlers: ShortcutHandlers): void {
  const handlersRef = useRef(handlers);
  handlersRef.current = handlers;

  useEffect(() => register(handlersRef), []);
}
