/**
 * Produces the encoded bytes of one thumbnail. The Tauri command answers with an
 * `ArrayBuffer`; the byte-array form is accepted so callers need no conversion.
 */
type ThumbnailFetcher = (
  slotId: number,
  width: number,
) => Promise<ArrayBuffer | Uint8Array<ArrayBuffer>>;

interface CacheEntry {
  promise: Promise<string>;
  retained: boolean;
  url?: string;
}

export interface ThumbnailCache {
  get: (slotId: number, width: number) => Promise<string>;
  release: () => void;
}

interface ThumbnailCacheOptions {
  fetcher: ThumbnailFetcher;
  capacity: number;
}

/**
 * Drops the cache's claim on an entry. A blob URL is only freed by revoking it,
 * and an entry whose fetch is still running revokes itself on arrival instead.
 */
function releaseEntry(entry: CacheEntry): void {
  entry.retained = false;
  if (entry.url) {
    URL.revokeObjectURL(entry.url);
  }
}

/**
 * Thumbnails are held as blob URLs, which stay alive until they are revoked
 * explicitly. The cache is therefore the only owner: it revokes on eviction and
 * on release, and `capacity` must stay above the number of cards that can be on
 * screen at once, so a visible card never has its URL revoked underneath it.
 */
export function createThumbnailCache({ fetcher, capacity }: ThumbnailCacheOptions): ThumbnailCache {
  if (capacity < 1) {
    throw new RangeError("Thumbnail cache capacity must be at least one");
  }

  // Insertion order is the LRU order: a hit re-inserts its key at the end.
  const entries = new Map<string, CacheEntry>();

  function evictLeastRecentlyUsed(): void {
    if (entries.size <= capacity) {
      return;
    }

    const oldestKey = entries.keys().next().value;
    if (oldestKey === undefined) {
      return;
    }

    const oldestEntry = entries.get(oldestKey);
    entries.delete(oldestKey);
    if (oldestEntry) {
      releaseEntry(oldestEntry);
    }
  }

  return {
    get(slotId, width) {
      const key = `${slotId}:${width}`;
      const cached = entries.get(key);
      if (cached) {
        entries.delete(key);
        entries.set(key, cached);
        return cached.promise;
      }

      // The real promise is assigned right below: its callbacks need `entry` to
      // already exist so they can record the URL they created.
      const entry: CacheEntry = { retained: true, promise: Promise.resolve("") };
      entry.promise = fetcher(slotId, width)
        .then((bytes) => {
          const url = URL.createObjectURL(new Blob([bytes], { type: "image/png" }));
          entry.url = url;
          if (!entry.retained) {
            // The entry was evicted while its fetch was still in flight, so
            // nothing will ever read this URL.
            URL.revokeObjectURL(url);
          }
          return url;
        })
        .catch((error: unknown) => {
          if (entries.get(key) === entry) {
            entries.delete(key);
          }
          throw error;
        });

      entries.set(key, entry);
      evictLeastRecentlyUsed();
      return entry.promise;
    },
    release() {
      for (const entry of entries.values()) {
        releaseEntry(entry);
      }
      entries.clear();
    },
  };
}
