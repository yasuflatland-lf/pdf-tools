import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { renderToString } from "react-dom/server";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ThumbnailCache } from "../../lib/thumbnail-cache";
import { ThumbnailCacheProvider, useThumbnailCache } from "../card/ThumbnailCacheProvider";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const mountedRoots: Root[] = [];

function CacheConsumer({ receive }: { receive: (cache: ThumbnailCache) => void }) {
  receive(useThumbnailCache());
  return null;
}

describe("ThumbnailCacheProvider", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(new Uint8Array([1, 2, 3]));
    Object.defineProperty(URL, "createObjectURL", {
      configurable: true,
      value: vi.fn(() => "blob:thumbnail"),
    });
    Object.defineProperty(URL, "revokeObjectURL", {
      configurable: true,
      value: vi.fn(),
    });
  });

  afterEach(async () => {
    await act(async () => {
      for (const root of mountedRoots.splice(0)) {
        root.unmount();
      }
    });
    document.body.replaceChildren();
    vi.restoreAllMocks();
  });

  it("throws when used outside the provider", () => {
    expect(() => renderToString(<CacheConsumer receive={() => {}} />)).toThrow(
      "useThumbnailCache must be used within ThumbnailCacheProvider",
    );
  });

  it("gives two consumers the same cache object", async () => {
    const caches: ThumbnailCache[] = [];
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);
    mountedRoots.push(root);

    await act(async () => {
      root.render(
        <ThumbnailCacheProvider>
          <CacheConsumer receive={(cache) => caches.push(cache)} />
          <CacheConsumer receive={(cache) => caches.push(cache)} />
        </ThumbnailCacheProvider>,
      );
    });

    expect(caches).toHaveLength(2);
    expect(caches[0]).toBe(caches[1]);
  });

  it("releases cached object URLs when the provider unmounts", async () => {
    let cache: ThumbnailCache | undefined;
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(
        <ThumbnailCacheProvider>
          <CacheConsumer receive={(value) => (cache = value)} />
        </ThumbnailCacheProvider>,
      );
    });
    await cache?.get(1, 200);

    await act(async () => root.unmount());

    expect(URL.revokeObjectURL).toHaveBeenCalledWith("blob:thumbnail");
  });
});
