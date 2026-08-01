import { useAddSources } from "./useAddSources";

/**
 * The empty state's two ways in, shown unfolded rather than behind a menu.
 * This is the only moment the user is deciding how to start, and folding the
 * folder picker away here is what would keep it from ever being found.
 */
export function AddSourcesButtons() {
  const { isIngesting, chooseFiles, chooseFolder } = useAddSources();

  return (
    <div className="mt-5 flex items-center justify-center gap-2.5">
      <button
        className="h-8 rounded-md bg-sky-600 px-4 text-sm font-semibold text-white hover:bg-sky-500 focus-visible:outline-2 focus-visible:outline-sky-400 disabled:cursor-not-allowed disabled:bg-slate-700 disabled:text-slate-400"
        disabled={isIngesting}
        onClick={() => void chooseFiles()}
        type="button"
      >
        Choose files…
      </button>
      <button
        className="h-8 rounded-md border border-slate-600 px-4 text-sm text-slate-200 hover:bg-slate-800 focus-visible:outline-2 focus-visible:outline-sky-500 disabled:cursor-not-allowed disabled:border-slate-800 disabled:text-slate-600"
        disabled={isIngesting}
        onClick={() => void chooseFolder()}
        type="button"
      >
        Choose folder…
      </button>
    </div>
  );
}
