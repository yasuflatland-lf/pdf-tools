import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it } from "vitest";
import { MergeProgressLine } from "../MergeProgressLine";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const mountedRoots: Root[] = [];

async function render(done: number, total: number): Promise<HTMLElement> {
  const container = document.createElement("div");
  document.body.append(container);
  const root = createRoot(container);
  mountedRoots.push(root);

  await act(async () => {
    root.render(<MergeProgressLine done={done} label="Merge progress" total={total} />);
  });

  return container;
}

function bar(container: HTMLElement): Element {
  const element = container.querySelector('[role="progressbar"]');
  if (!element) {
    throw new Error("No progressbar was rendered");
  }
  return element;
}

describe("MergeProgressLine", () => {
  afterEach(async () => {
    await act(async () => {
      for (const root of mountedRoots.splice(0)) {
        root.unmount();
      }
    });
    document.body.replaceChildren();
  });

  it("reports the completed percentage", async () => {
    const container = await render(3, 4);

    expect(bar(container).getAttribute("aria-valuenow")).toBe("75");
    expect(bar(container).getAttribute("aria-label")).toBe("Merge progress");
  });

  it("reports zero before the total is known", async () => {
    const container = await render(0, 0);

    expect(bar(container).getAttribute("aria-valuenow")).toBe("0");
  });

  it("never exceeds one hundred", async () => {
    const container = await render(9, 4);

    expect(bar(container).getAttribute("aria-valuenow")).toBe("100");
  });
});
