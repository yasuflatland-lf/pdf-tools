import type { SourceStatusDto } from "../bindings/SourceStatusDto";
import { countLabel } from "../lib/format";
import { usePlanStore } from "../store/plan-store";
import { DropZone } from "./DropZone";
import { Toolbar } from "./Toolbar";

function statusLabel(status: SourceStatusDto): string {
  if (status.kind === "ready") {
    return "Ready";
  }
  if (status.kind === "encrypted") {
    return "Encrypted";
  }
  return `Unreadable: ${status.reason}`;
}

export function AppShell() {
  const sources = usePlanStore((state) => state.sources);

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-slate-950 text-slate-100">
      <header className="bg-slate-950 px-6 py-5">
        <h1 className="text-2xl font-semibold tracking-tight">PDF Tools</h1>
        <p className="mt-1 text-sm text-slate-400">Combine PDFs and images into a single PDF.</p>
      </header>

      <Toolbar />

      <DropZone>
        <main className="h-full overflow-y-auto px-6 py-6">
          {sources.length === 0 ? (
            <div className="grid min-h-48 place-items-center rounded-xl border border-dashed border-slate-700 text-center">
              <div>
                <p className="font-medium text-slate-200">Drop PDFs or images here</p>
                <p className="mt-1 text-sm text-slate-500">
                  Source files will appear in this document.
                </p>
              </div>
            </div>
          ) : (
            <ul className="space-y-3" aria-label="Source files">
              {sources.map((source) => (
                <li
                  key={source.id}
                  className="rounded-xl border border-slate-800 bg-slate-900 px-4 py-3"
                >
                  <p className="truncate font-medium text-slate-100">{source.file_name}</p>
                  <p className="mt-1 text-sm text-slate-400">
                    {countLabel(source.page_count, "page")} · {statusLabel(source.status)}
                  </p>
                </li>
              ))}
            </ul>
          )}
        </main>
      </DropZone>
    </div>
  );
}
