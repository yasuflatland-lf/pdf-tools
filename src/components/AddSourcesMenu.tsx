import { FileText, FolderOpen, Plus } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useAddSources } from "./useAddSources";

const ITEM_CLASS =
  "flex w-full items-center gap-2.5 rounded px-2.5 py-1.5 text-left text-slate-200 hover:bg-slate-700 focus-visible:bg-slate-700 focus-visible:outline-none disabled:cursor-not-allowed disabled:text-slate-600";

/** Moves focus between items so the menu is usable without a pointer. */
function onMenuKeyDown(event: React.KeyboardEvent<HTMLDivElement>) {
  if (event.key !== "ArrowDown" && event.key !== "ArrowUp") {
    return;
  }

  event.preventDefault();
  const items = Array.from(
    event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="menuitem"]'),
  );
  const current = items.indexOf(document.activeElement as HTMLButtonElement);
  const step = event.key === "ArrowDown" ? 1 : -1;
  const next = (current + step + items.length) % items.length;
  items[next]?.focus();
}

/**
 * The toolbar's persistent way into the document.
 *
 * Both actions fold into a menu here purely for room. The empty state shows
 * them unfolded, so by the time anyone reaches this control they already know
 * a folder is an option -- folding costs a click, not discoverability.
 */
export function AddSourcesMenu() {
  const [isOpen, setIsOpen] = useState(false);
  const { isIngesting, chooseFiles, chooseFolder } = useAddSources();
  const containerRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);

  const close = useCallback((restoreFocus: boolean) => {
    setIsOpen(false);
    if (restoreFocus) {
      triggerRef.current?.focus();
    }
  }, []);

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    const onPointerDown = (event: MouseEvent) => {
      if (!containerRef.current?.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") {
        return;
      }
      // Stopped here so Escape closing the menu does not also clear the page
      // selection behind it: one key press, one effect.
      event.stopPropagation();
      close(true);
    };

    document.addEventListener("mousedown", onPointerDown);
    document.addEventListener("keydown", onKeyDown, true);
    return () => {
      document.removeEventListener("mousedown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown, true);
    };
  }, [isOpen, close]);

  const run = (action: () => Promise<void>) => {
    close(false);
    void action();
  };

  return (
    <div className="relative" ref={containerRef}>
      <button
        aria-expanded={isOpen}
        aria-haspopup="menu"
        aria-label="Add files or a folder"
        className="flex h-7 items-center gap-1.5 rounded-md border border-slate-600 pl-2 pr-2.5 text-slate-200 hover:bg-slate-800 focus-visible:outline-2 focus-visible:outline-sky-500 disabled:cursor-not-allowed disabled:border-slate-800 disabled:text-slate-600"
        disabled={isIngesting}
        onClick={() => setIsOpen((open) => !open)}
        ref={triggerRef}
        type="button"
      >
        <Plus aria-hidden="true" size={15} strokeWidth={2} />
        Add
      </button>

      {isOpen && (
        <div
          className="absolute left-0 top-9 z-10 w-44 rounded-lg border border-slate-700 bg-slate-800 p-1 text-xs shadow-lg"
          onKeyDown={onMenuKeyDown}
          role="menu"
        >
          <button
            className={ITEM_CLASS}
            onClick={() => run(chooseFiles)}
            role="menuitem"
            type="button"
          >
            <FileText aria-hidden="true" size={14} strokeWidth={1.8} />
            Files…
          </button>
          <button
            className={ITEM_CLASS}
            onClick={() => run(chooseFolder)}
            role="menuitem"
            type="button"
          >
            <FolderOpen aria-hidden="true" size={14} strokeWidth={1.8} />
            Folder…
          </button>
        </div>
      )}
    </div>
  );
}
