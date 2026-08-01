import type { ReactNode } from "react";

/**
 * The amber note a card shows when something about its source is wrong. One
 * component so a thumbnail failure and an unreadable file cannot end up
 * looking like two different kinds of problem.
 */
export function Notice({ children }: { children: ReactNode }) {
  return (
    <p className="mt-2 rounded-md border border-amber-700/60 bg-amber-950/50 px-2 py-1 text-xs text-amber-100">
      {children}
    </p>
  );
}
