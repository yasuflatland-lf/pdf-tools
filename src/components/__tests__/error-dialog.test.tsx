import { act, type ReactElement } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { SourceFileDto } from "../../bindings/SourceFileDto";
import { blamedFiles, ErrorDialog } from "../ErrorDialog";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const mountedRoots: Root[] = [];

function source(
  id: number,
  fileName: string,
  path: string,
  status: SourceFileDto["status"],
): SourceFileDto {
  return {
    id,
    path,
    file_name: fileName,
    kind: "pdf",
    grouping: "ungrouped",
    page_count: status.kind === "ready" ? 1 : 0,
    status,
  };
}

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

describe("blamedFiles", () => {
  it("picks the file named in the message and any non-ready source", () => {
    const sources = [
      source(1, "good.pdf", "/documents/good.pdf", { kind: "ready" }),
      source(2, "named.pdf", "/documents/named.pdf", { kind: "ready" }),
      source(3, "locked.pdf", "/documents/locked.pdf", { kind: "encrypted" }),
    ];

    expect(blamedFiles("failed to read /documents/named.pdf", sources)).toEqual([
      "named.pdf",
      "locked.pdf",
    ]);
  });
});

describe("ErrorDialog", () => {
  it("exposes an alert dialog and lists the blamed file names", async () => {
    const container = await render(
      <ErrorDialog
        files={["named.pdf", "locked.pdf"]}
        message="Merge could not read a source"
        onClose={() => {}}
      />,
    );
    const dialog = container.querySelector('[role="alertdialog"]');
    const headingId = dialog?.getAttribute("aria-labelledby");

    expect(dialog).not.toBeNull();
    expect(dialog?.getAttribute("aria-modal")).toBe("true");
    expect(headingId).not.toBeNull();
    expect(document.getElementById(headingId as string)?.textContent).toBe("結合できませんでした");
    expect(container.textContent).toContain("Merge could not read a source");
    expect(container.textContent).toContain("named.pdf");
    expect(container.textContent).toContain("locked.pdf");
  });

  it("calls onClose from the close button and Escape", async () => {
    const onClose = vi.fn();
    const container = await render(
      <ErrorDialog files={[]} message="Merge failed" onClose={onClose} />,
    );
    const closeButton = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent === "閉じる",
    );

    // The dialog takes focus so it can be dismissed without the mouse.
    expect(document.activeElement).toBe(closeButton);

    await act(async () => {
      closeButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(onClose).toHaveBeenCalledTimes(1);

    await act(async () => {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    });
    expect(onClose).toHaveBeenCalledTimes(2);
  });
});
