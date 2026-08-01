import { createContext, useContext, useEffect, useState, type ReactNode } from "react";
import { rasterizeSlot } from "../../lib/tauri-api";
import { createThumbnailCache, type ThumbnailCache } from "../../lib/thumbnail-cache";

/**
 * Capacity has to stay above the number of cards that can be on screen at
 * once, or a visible card has its blob URL revoked underneath it. Both views
 * are held at once now -- each keys on its own thumbnail width -- so the
 * budget covers a full screen of either.
 */
const CACHE_CAPACITY = 200;

const ThumbnailCacheContext = createContext<ThumbnailCache | null>(null);

export function useThumbnailCache(): ThumbnailCache {
  const cache = useContext(ThumbnailCacheContext);
  if (!cache) {
    throw new Error("useThumbnailCache must be used within ThumbnailCacheProvider");
  }
  return cache;
}

/**
 * Owns the thumbnail cache for as long as the document is open. It sits above
 * the grid/list switch so that toggling the view does not throw away every
 * rendered page and rasterize them all again.
 */
export function ThumbnailCacheProvider({ children }: { children: ReactNode }) {
  const [cache] = useState(() =>
    createThumbnailCache({ fetcher: rasterizeSlot, capacity: CACHE_CAPACITY }),
  );

  useEffect(() => () => cache.release(), [cache]);

  return <ThumbnailCacheContext value={cache}>{children}</ThumbnailCacheContext>;
}
