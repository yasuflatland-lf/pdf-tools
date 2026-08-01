import { X } from "lucide-react";
import { useUiStore } from "../store/ui-store";

/**
 * A one-line report about the last add attempt.
 *
 * It sits above the document rather than inside the empty state, because the
 * toolbar can start an add while the document already has pages -- and then
 * the empty state is not mounted to render into. With an empty plan it lands
 * directly above the drop prompt, which is where the report is expected.
 *
 * Nothing dismisses it on a timer: a message that disappears by itself is
 * unreadable to anyone who looked away while the merge ran.
 */
export function SourceNotice() {
  const notice = useUiStore((state) => state.sourceNotice);

  if (notice === null) {
    return null;
  }

  return (
    <div
      className="flex items-center gap-3 rounded-lg border border-slate-700 bg-slate-900 px-3.5 py-2 text-sm text-slate-300"
      role="status"
    >
      <span className="flex-1">{notice}</span>
      <button
        aria-label="Dismiss"
        className="grid size-6 place-items-center rounded text-slate-500 hover:bg-slate-800 hover:text-slate-300 focus-visible:outline-2 focus-visible:outline-sky-500"
        onClick={() => useUiStore.getState().setSourceNotice(null)}
        type="button"
      >
        <X aria-hidden="true" size={14} strokeWidth={1.8} />
      </button>
    </div>
  );
}
