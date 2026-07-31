import { useEffect, useId, useRef, type ReactElement } from "react";
import type { SourceFileDto } from "../bindings/SourceFileDto";

/**
 * Names the files a failed merge should point at. The backend reports the
 * offending path inside the error text, so a source is blamed when its path
 * appears there; sources that were already known to be unmergeable are listed
 * too, because they are the usual reason a merge cannot produce every page.
 */
export function blamedFiles(message: string, sources: SourceFileDto[]): string[] {
  const files: string[] = [];
  const seen = new Set<string>();

  for (const source of sources) {
    const isBlamed = source.status.kind !== "ready" || message.includes(source.path);
    if (isBlamed && !seen.has(source.file_name)) {
      files.push(source.file_name);
      seen.add(source.file_name);
    }
  }

  return files;
}

/**
 * Reports a failure the user cannot be expected to diagnose from a toolbar
 * label alone. It is modal on purpose: a merge that produced no file must not
 * be mistaken for one that quietly succeeded.
 */
export function ErrorDialog({
  files,
  message,
  onClose,
}: {
  files: string[];
  message: string;
  onClose: () => void;
}): ReactElement {
  const headingId = useId();
  const filesLabelId = useId();
  const closeRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  // Focus moves into the dialog so that dismissing it never requires the mouse
  // and so that the next Tab continues from here rather than from the toolbar
  // behind the overlay.
  useEffect(() => {
    closeRef.current?.focus();
  }, []);

  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-slate-950/80 p-6">
      <section
        className="w-full max-w-lg rounded-xl border border-red-800/70 bg-slate-900 p-6 shadow-2xl"
        role="alertdialog"
        aria-labelledby={headingId}
        aria-modal="true"
      >
        <h2 id={headingId} className="text-lg font-semibold text-red-200">
          Merge failed
        </h2>
        <p className="mt-3 break-words text-sm text-slate-300">{message}</p>
        {files.length > 0 && (
          <div className="mt-4">
            <p id={filesLabelId} className="text-sm font-medium text-slate-200">
              Affected files
            </p>
            <ul
              className="mt-2 list-disc space-y-1 pl-5 text-sm text-slate-300"
              aria-labelledby={filesLabelId}
            >
              {files.map((file) => (
                <li key={file}>{file}</li>
              ))}
            </ul>
          </div>
        )}
        <div className="mt-6 flex justify-end">
          <button
            ref={closeRef}
            className="rounded-md bg-sky-600 px-4 py-2 text-sm font-medium text-white hover:bg-sky-500 focus-visible:ring-2 focus-visible:ring-sky-300 focus-visible:outline-none"
            onClick={onClose}
            type="button"
          >
            Close
          </button>
        </div>
      </section>
    </div>
  );
}
