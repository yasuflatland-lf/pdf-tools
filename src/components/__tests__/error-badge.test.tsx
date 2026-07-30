import { act, type ReactElement } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it } from "vitest";
import { ErrorBadge } from "../ErrorBadge";
import { SourceErrorCard } from "../PageCard";

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

describe("ErrorBadge", () => {
  it("shows a password-required badge for encrypted sources", async () => {
    const container = await render(<ErrorBadge status={{ kind: "encrypted" }} />);

    expect(container.textContent).toMatch(/パスワード/);
  });

  it("shows the reason for unreadable sources", async () => {
    const container = await render(
      <ErrorBadge status={{ kind: "unreadable", reason: "broken xref" }} />,
    );

    expect(container.textContent).toContain("broken xref");
  });

  it("renders nothing for ready sources", async () => {
    const container = await render(<ErrorBadge status={{ kind: "ready" }} />);

    expect(container.innerHTML).toBe("");
  });

  it("renders a dimmed source error card with the file name and badge", async () => {
    const container = await render(
      <SourceErrorCard
        fileName="damaged.pdf"
        status={{ kind: "unreadable", reason: "broken xref" }}
      />,
    );
    const card = container.querySelector("article");

    expect(card?.classList.contains("opacity-60")).toBe(true);
    expect(container.textContent).toContain("damaged.pdf");
    expect(container.textContent).toContain("broken xref");
  });
});
