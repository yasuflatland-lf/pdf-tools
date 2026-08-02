import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ToggleButton } from "../card/ToggleButton";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const mountedRoots: Root[] = [];

async function renderToggle(
  collapsed: boolean,
  onToggle = vi.fn(),
  onAncestorKeyDown = vi.fn(),
): Promise<{ button: HTMLButtonElement; container: HTMLElement }> {
  const container = document.createElement("div");
  document.body.append(container);
  const root = createRoot(container);
  mountedRoots.push(root);

  await act(async () => {
    root.render(
      <div onKeyDown={onAncestorKeyDown}>
        <ToggleButton collapsed={collapsed} fileName="report.pdf" onToggle={onToggle} />
      </div>,
    );
  });

  const button = container.querySelector("button");
  if (!button) throw new Error("Expected toggle button");
  return { button, container };
}

afterEach(async () => {
  await act(async () => {
    for (const root of mountedRoots.splice(0)) {
      root.unmount();
    }
  });
  document.body.replaceChildren();
});

describe("ToggleButton", () => {
  it.each([
    [true, "Expand"],
    [false, "Collapse"],
  ])("renders %s state as %s", async (collapsed, label) => {
    const { button } = await renderToggle(collapsed);

    expect(button.textContent).toBe(label);
    expect(button.getAttribute("aria-label")).toBe(`${label} report.pdf`);
  });

  it("fires onToggle when clicked", async () => {
    const onToggle = vi.fn();
    const { button } = await renderToggle(true, onToggle);

    await act(async () => {
      button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(onToggle).toHaveBeenCalledOnce();
  });

  it("stops a keydown from reaching an ancestor", async () => {
    const onKeyDown = vi.fn();
    const { button } = await renderToggle(true, vi.fn(), onKeyDown);

    await act(async () => {
      button.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, key: "Enter" }));
    });

    expect(onKeyDown).not.toHaveBeenCalled();
  });
});
