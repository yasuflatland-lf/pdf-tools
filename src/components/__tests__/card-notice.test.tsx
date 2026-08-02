import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it } from "vitest";
import { Notice } from "../card/Notice";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const mountedRoots: Root[] = [];

afterEach(async () => {
  await act(async () => {
    for (const root of mountedRoots.splice(0)) {
      root.unmount();
    }
  });
  document.body.replaceChildren();
});

describe("Notice", () => {
  it("renders its children and carries the amber classes exactly once", async () => {
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);
    mountedRoots.push(root);

    await act(async () => {
      root.render(<Notice>Thumbnail unavailable</Notice>);
    });

    expect(container.textContent).toBe("Thumbnail unavailable");
    // The point of the extraction is that the amber class string lives in one
    // place, so the assertion pins the shape -- one paragraph, styled amber --
    // rather than restating the string and becoming the second copy.
    expect(container.querySelectorAll("p")).toHaveLength(1);
    expect(container.querySelectorAll('[class*="amber"]')).toHaveLength(1);
    expect(container.querySelector('[class*="amber"]')?.tagName).toBe("P");
  });
});
