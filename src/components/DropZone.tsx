import { getCurrentWebview } from "@tauri-apps/api/webview";
import type { PropsWithChildren } from "react";
import { useEffect, useState } from "react";
import { useAddSources } from "./useAddSources";

export function DropZone({ children }: PropsWithChildren) {
  const [isDragging, setIsDragging] = useState(false);
  const { addPaths } = useAddSources();

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    void getCurrentWebview()
      .onDragDropEvent(async (event) => {
        const { payload } = event;

        if (payload.type === "enter" || payload.type === "over") {
          setIsDragging(true);
          return;
        }

        setIsDragging(false);
        if (payload.type !== "drop" || payload.paths.length === 0) {
          return;
        }

        // A drop goes through the same procedure as the buttons, so a folder
        // cannot behave one way when dropped and another when picked. It also
        // reports its own failures, which this listener could not do: it has
        // nowhere to return one to.
        await addPaths(payload.paths);
      })
      .then((stopListening) => {
        if (disposed) {
          stopListening();
        } else {
          unlisten = stopListening;
        }
      })
      .catch((error: unknown) => {
        console.error("failed to subscribe to drag and drop events", error);
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
    // `addPaths` is stable across renders, so the listener registers once.
  }, [addPaths]);

  return (
    <div
      className={`min-h-0 flex-1 border-2 transition-colors ${
        isDragging ? "border-sky-400 bg-sky-400/10" : "border-transparent"
      }`}
    >
      {children}
    </div>
  );
}
