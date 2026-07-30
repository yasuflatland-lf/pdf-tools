import { act } from "react";
import { createRoot } from "react-dom/client";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "../App";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));

// React requires this flag to be set when `act` is used outside a test renderer.
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

async function renderApp(): Promise<HTMLElement> {
  const container = document.createElement("div");
  document.body.append(container);
  await act(async () => {
    createRoot(container).render(<App />);
  });
  return container;
}

describe("App", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("shows the PDFium version reported by the backend", async () => {
    invoke.mockResolvedValue("FPDF API V7881");

    const container = await renderApp();

    expect(invoke).toHaveBeenCalledWith("pdfium_health");
    expect(container.textContent).toContain("PDFium: FPDF API V7881");
  });

  it("shows the reason when PDFium is unavailable", async () => {
    invoke.mockRejectedValue("PDFium engine is unavailable: no such file");

    const container = await renderApp();

    expect(container.textContent).toContain("PDFium engine is unavailable: no such file");
  });
});
