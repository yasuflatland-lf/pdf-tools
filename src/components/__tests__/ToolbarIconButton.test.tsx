import { Undo2 } from "lucide-react";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ToolbarIconButton } from "../ToolbarIconButton";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const mountedRoots: Root[] = [];

async function render(element: React.ReactElement): Promise<HTMLElement> {
  const container = document.createElement("div");
  document.body.append(container);
  const root = createRoot(container);
  mountedRoots.push(root);

  await act(async () => {
    root.render(element);
  });

  return container;
}

function onlyButton(container: HTMLElement): HTMLButtonElement {
  const button = container.querySelector("button");
  if (!button) {
    throw new Error("No button was rendered");
  }
  return button;
}

describe("ToolbarIconButton", () => {
  afterEach(async () => {
    await act(async () => {
      for (const root of mountedRoots.splice(0)) {
        root.unmount();
      }
    });
    document.body.replaceChildren();
  });

  it("names itself and puts the shortcut in the tooltip", async () => {
    const container = await render(
      <ToolbarIconButton icon={Undo2} label="Undo" onClick={() => {}} shortcut="⌘Z" />,
    );
    const button = onlyButton(container);

    expect(button.getAttribute("aria-label")).toBe("Undo");
    expect(button.getAttribute("title")).toBe("Undo (⌘Z)");
    // The glyph is decoration: the name lives on the button, not on the svg.
    expect(button.querySelector("svg")?.getAttribute("aria-hidden")).toBe("true");
  });

  it("uses the bare label as the tooltip when there is no shortcut", async () => {
    const container = await render(<ToolbarIconButton icon={Undo2} label="Undo" onClick={() => {}} />);

    expect(onlyButton(container).getAttribute("title")).toBe("Undo");
  });

  it("does not fire while disabled", async () => {
    const onClick = vi.fn();
    const container = await render(
      <ToolbarIconButton disabled icon={Undo2} label="Undo" onClick={onClick} />,
    );
    const button = onlyButton(container);

    expect(button.disabled).toBe(true);

    await act(async () => {
      button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await Promise.resolve();
    });

    expect(onClick).not.toHaveBeenCalled();
  });
});
