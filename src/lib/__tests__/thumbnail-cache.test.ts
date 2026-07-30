import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createThumbnailCache } from "../thumbnail-cache";

describe("createThumbnailCache", () => {
  beforeEach(() => {
    Object.defineProperty(URL, "createObjectURL", {
      configurable: true,
      value: vi.fn(() => `blob:thumbnail-${Math.random()}`),
    });
    Object.defineProperty(URL, "revokeObjectURL", {
      configurable: true,
      value: vi.fn(),
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("fetches a thumbnail once and serves the cached blob afterwards", async () => {
    const fetcher = vi.fn().mockResolvedValue(new Uint8Array([1, 2, 3]));
    const cache = createThumbnailCache({ fetcher, capacity: 10 });

    await cache.get(1, 200);
    await cache.get(1, 200);

    expect(fetcher).toHaveBeenCalledTimes(1);
  });

  it("deduplicates concurrent requests for the same thumbnail", async () => {
    const fetcher = vi.fn().mockResolvedValue(new Uint8Array([1, 2, 3]));
    const cache = createThumbnailCache({ fetcher, capacity: 10 });

    await Promise.all([cache.get(1, 200), cache.get(1, 200)]);

    expect(fetcher).toHaveBeenCalledTimes(1);
  });

  it("evicts the least recently used entry and revokes its object URL", async () => {
    const revoke = vi.spyOn(URL, "revokeObjectURL");
    const fetcher = vi.fn().mockResolvedValue(new Uint8Array([1, 2, 3]));
    const cache = createThumbnailCache({ fetcher, capacity: 2 });

    await cache.get(1, 200);
    await cache.get(2, 200);
    await cache.get(1, 200);
    await cache.get(3, 200);

    expect(revoke).toHaveBeenCalledTimes(1);
  });

  // Counting revocations cannot tell LRU apart from plain insertion order: both
  // discard exactly one entry here. Naming the revoked URL is what pins the
  // eviction to the entry that went untouched.
  it("keeps the re-read entry and evicts the one that went untouched", async () => {
    const revoke = vi.spyOn(URL, "revokeObjectURL");
    const fetcher = vi.fn().mockResolvedValue(new Uint8Array([1, 2, 3]));
    const cache = createThumbnailCache({ fetcher, capacity: 2 });

    const first = await cache.get(1, 200);
    const second = await cache.get(2, 200);
    await cache.get(1, 200);
    await cache.get(3, 200);

    expect(revoke).toHaveBeenCalledWith(second);
    expect(revoke).not.toHaveBeenCalledWith(first);
  });

  it("releases every cached object URL", async () => {
    const revoke = vi.spyOn(URL, "revokeObjectURL");
    const fetcher = vi.fn().mockResolvedValue(new Uint8Array([1, 2, 3]));
    const cache = createThumbnailCache({ fetcher, capacity: 10 });

    await cache.get(1, 200);
    await cache.get(2, 200);
    cache.release();

    expect(revoke).toHaveBeenCalledTimes(2);
  });
});
