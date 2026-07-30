import { getCurrentWebview } from "@tauri-apps/api/webview";
import type { PropsWithChildren } from "react";
import { useEffect, useState } from "react";
import { addSources } from "../lib/tauri-api";
import { usePlanStore } from "../store/plan-store";

export function DropZone({ children }: PropsWithChildren) {
  const [isDragging, setIsDragging] = useState(false);

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

        try {
          const snapshot = await addSources(payload.paths);
          usePlanStore.getState().setSnapshot(snapshot);
        } catch (error) {
          // A webview listener has nowhere to return a failure to, and an
          // unhandled rejection would be invisible. Surfacing this in the UI
          // is Task 29.
          console.error("add_sources failed", error);
        }
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
  }, []);

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
