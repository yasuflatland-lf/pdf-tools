import { useEffect, useState } from "react";
import type { ThumbnailCache } from "../lib/thumbnail-cache";

/**
 * Requests one slot's thumbnail and reports what came back. Both card shapes
 * need exactly this, and keeping two copies of it is how they drifted apart:
 * the grid and the list rendered the same failure differently.
 */
export function useThumbnail(
  cache: ThumbnailCache,
  slotId: number,
  width: number,
): { url: string | undefined; failed: boolean } {
  const [url, setUrl] = useState<string>();
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let active = true;
    setUrl(undefined);
    setFailed(false);

    void cache
      .get(slotId, width)
      .then((resolved) => {
        if (active && resolved) {
          setUrl(resolved);
        }
      })
      .catch(() => {
        if (active) {
          setFailed(true);
        }
      });

    return () => {
      active = false;
    };
  }, [cache, slotId, width]);

  return { url, failed };
}
