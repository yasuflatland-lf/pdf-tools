import { RotateCcw } from "lucide-react";
import { act, type ReactElement } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CardControl, CardIconControl } from "../card/CardControl";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const mountedRoots: Root[] = [];

async function render(element: ReactElement): Promise<HTMLElement> {
  const container = document.createElement("div");
  document.body.append(container);
  const root = createRoot(container);
  mountedRoots.push(root);

  await act(async () => {
    root.render(element);
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
});

describe("card controls", () => {
  it("stops a pointerdown from reaching an ancestor", async () => {
    const onPointerDown = vi.fn();
    const container = await render(
      <div onPointerDown={onPointerDown}>
        <CardControl label="Remove report.pdf" onPress={() => {}}>
          Remove
        </CardControl>
        <CardIconControl
          icon={RotateCcw}
          label="Rotate left report.pdf"
          onPress={() => {}}
          title="Rotate left"
        />
      </div>,
    );

    for (const button of container.querySelectorAll("button")) {
      await act(async () => {
        button.dispatchEvent(new MouseEvent("pointerdown", { bubbles: true }));
      });
    }

    expect(onPointerDown).not.toHaveBeenCalled();
  });

  it("stops a keydown from reaching an ancestor", async () => {
    const onKeyDown = vi.fn();
    const container = await render(
      <div onKeyDown={onKeyDown}>
        <CardControl label="Remove report.pdf" onPress={() => {}}>
          Remove
        </CardControl>
        <CardIconControl
          icon={RotateCcw}
          label="Rotate left report.pdf"
          onPress={() => {}}
          title="Rotate left"
        />
      </div>,
    );

    for (const button of container.querySelectorAll("button")) {
      await act(async () => {
        button.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, key: "Enter" }));
      });
    }

    expect(onKeyDown).not.toHaveBeenCalled();
  });

  it("names the action alone in the tooltip and the card in the accessible name", async () => {
    const container = await render(
      <CardIconControl
        icon={RotateCcw}
        label="Rotate left report.pdf"
        onPress={() => {}}
        title="Rotate left"
      />,
    );
    const button = container.querySelector("button");

    expect(button?.getAttribute("title")).toBe("Rotate left");
    expect(button?.getAttribute("aria-label")).toBe("Rotate left report.pdf");
  });
});
