import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ThumbnailCache } from "../../lib/thumbnail-cache";
import { PageCard } from "../PageCard";
import type { CardViewProps } from "../card/CardProps";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

// The thumbnail never arrives, which leaves the placeholder in place and keeps
// these tests about the controls drawn over it.
const cache: ThumbnailCache = {
  get: vi.fn(() => new Promise<string>(() => {})),
  release: vi.fn(),
};

const mountedRoots: Root[] = [];

function props(overrides: Partial<CardViewProps> = {}): CardViewProps {
  return {
    cache,
    collapsed: false,
    fileName: "report.pdf",
    pageCount: 1,
    pageNumber: 1,
    rotation: 0,
    selected: false,
    slotId: 1,
    thumbnailWidth: 360,
    ...overrides,
  };
}

async function renderCard(cardProps: CardViewProps): Promise<HTMLElement> {
  const container = document.createElement("div");
  document.body.append(container);
  const root = createRoot(container);
  mountedRoots.push(root);

  await act(async () => {
    root.render(<PageCard {...cardProps} />);
  });

  return container;
}

afterEach(async () => {
  await act(async () => {
    for (const root of mountedRoots.splice(0)) {
      root.unmount();
    }
  });
  document.body.replaceChildren();
  vi.clearAllMocks();
});

describe("PageCard rotate controls", () => {
  it("renders no rotate control when no rotation handler is given", async () => {
    const container = await renderCard(props());

    // `onRotate` is optional on the shared contract, so a caller may omit it.
    // A button that cannot rotate anything must not be offered.
    expect(container.querySelector('[aria-label^="Rotate left "]')).toBeNull();
    expect(container.querySelector('[aria-label^="Rotate right "]')).toBeNull();
  });

  it("renders both rotate controls when a rotation handler is given", async () => {
    const onRotate = vi.fn();
    const container = await renderCard(props({ onRotate }));

    const left = container.querySelector<HTMLButtonElement>(
      '[aria-label="Rotate left report.pdf"]',
    );
    const right = container.querySelector<HTMLButtonElement>(
      '[aria-label="Rotate right report.pdf"]',
    );

    await act(async () => {
      left?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      right?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(onRotate.mock.calls).toEqual([[-1], [1]]);
  });
});
