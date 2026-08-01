import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { usePlanStore } from "../../store/plan-store";
import { useUiStore } from "../../store/ui-store";
import { useAddSources } from "../useAddSources";

const { addSources, ask, expandPaths, open } = vi.hoisted(() => ({
  addSources: vi.fn(),
  ask: vi.fn(),
  expandPaths: vi.fn(),
  open: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({ ask, open }));
vi.mock("../../lib/tauri-api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../lib/tauri-api")>()),
  addSources,
  expandPaths,
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const emptySnapshot = { slots: [], sources: [], can_undo: false, can_redo: false };

const mountedRoots: Root[] = [];

type Controller = ReturnType<typeof useAddSources>;

/** Mounts the hook and hands back its latest return value. */
async function renderHook(): Promise<() => Controller> {
  let latest: Controller | undefined;

  function Probe() {
    latest = useAddSources();
    return null;
  }

  const container = document.createElement("div");
  document.body.append(container);
  const root = createRoot(container);
  mountedRoots.push(root);

  await act(async () => {
    root.render(<Probe />);
  });

  return () => {
    if (!latest) {
      throw new Error("the hook did not render");
    }
    return latest;
  };
}

describe("useAddSources", () => {
  beforeEach(() => {
    addSources.mockReset();
    addSources.mockResolvedValue(emptySnapshot);
    ask.mockReset();
    expandPaths.mockReset();
    open.mockReset();
    useUiStore.setState({ isIngesting: false, sourceNotice: null });
  });

  afterEach(async () => {
    await act(async () => {
      for (const root of mountedRoots.splice(0)) {
        root.unmount();
      }
    });
    document.body.replaceChildren();
    usePlanStore.getState().setSnapshot(emptySnapshot);
    useUiStore.setState({ isIngesting: false, sourceNotice: null });
    vi.restoreAllMocks();
  });

  it("asks the picker for supported files only, then adds what it expands", async () => {
    open.mockResolvedValue(["/a/report.pdf"]);
    expandPaths.mockResolvedValue(["/a/report.pdf"]);
    const controller = await renderHook();

    await act(async () => {
      await controller().chooseFiles();
    });

    expect(open).toHaveBeenCalledWith({
      multiple: true,
      filters: [{ name: "PDFs and images", extensions: ["pdf", "jpg", "jpeg", "png", "gif"] }],
    });
    expect(expandPaths).toHaveBeenCalledWith(["/a/report.pdf"]);
    expect(addSources).toHaveBeenCalledWith(["/a/report.pdf"]);
  });

  it("asks the picker for a directory when choosing a folder", async () => {
    open.mockResolvedValue("/scans");
    expandPaths.mockResolvedValue(["/scans/a.pdf"]);
    const controller = await renderHook();

    await act(async () => {
      await controller().chooseFolder();
    });

    expect(open).toHaveBeenCalledWith({ directory: true });
    expect(expandPaths).toHaveBeenCalledWith(["/scans"]);
    expect(addSources).toHaveBeenCalledWith(["/scans/a.pdf"]);
  });

  it("does nothing when the picker is cancelled", async () => {
    open.mockResolvedValue(null);
    const controller = await renderHook();

    await act(async () => {
      await controller().chooseFiles();
    });

    expect(expandPaths).not.toHaveBeenCalled();
    expect(addSources).not.toHaveBeenCalled();
    expect(useUiStore.getState().sourceNotice).toBeNull();
  });

  it("names the folder in a notice when it holds nothing supported", async () => {
    open.mockResolvedValue("/home/me/Scans");
    expandPaths.mockResolvedValue([]);
    const controller = await renderHook();

    await act(async () => {
      await controller().chooseFolder();
    });

    expect(useUiStore.getState().sourceNotice).toBe('No PDFs or images found in "Scans".');
    expect(addSources).not.toHaveBeenCalled();
  });

  it("falls back to a plural notice when several folders came up empty", async () => {
    expandPaths.mockResolvedValue([]);
    const controller = await renderHook();

    await act(async () => {
      await controller().addPaths(["/a/one", "/a/two"]);
    });

    expect(useUiStore.getState().sourceNotice).toBe(
      "No PDFs or images found in the selected folders.",
    );
  });

  it("confirms before adding more files than the threshold", async () => {
    const many = Array.from({ length: 201 }, (_, index) => `/scans/page${index}.pdf`);
    open.mockResolvedValue("/scans");
    expandPaths.mockResolvedValue(many);
    ask.mockResolvedValue(false);
    const controller = await renderHook();

    await act(async () => {
      await controller().chooseFolder();
    });

    expect(ask).toHaveBeenCalledWith('Add 201 files from "scans"?', {
      title: "Add files",
      kind: "info",
    });
    expect(addSources).not.toHaveBeenCalled();
  });

  it("adds them when the confirmation is accepted", async () => {
    const many = Array.from({ length: 201 }, (_, index) => `/scans/page${index}.pdf`);
    open.mockResolvedValue("/scans");
    expandPaths.mockResolvedValue(many);
    ask.mockResolvedValue(true);
    const controller = await renderHook();

    await act(async () => {
      await controller().chooseFolder();
    });

    expect(addSources).toHaveBeenCalledWith(many);
  });

  it("does not confirm at exactly the threshold", async () => {
    const exactly = Array.from({ length: 200 }, (_, index) => `/scans/page${index}.pdf`);
    open.mockResolvedValue("/scans");
    expandPaths.mockResolvedValue(exactly);
    const controller = await renderHook();

    await act(async () => {
      await controller().chooseFolder();
    });

    expect(ask).not.toHaveBeenCalled();
    expect(addSources).toHaveBeenCalledWith(exactly);
  });

  it("reports a failed add in the notice instead of swallowing it", async () => {
    expandPaths.mockResolvedValue(["/a/report.pdf"]);
    addSources.mockRejectedValue("the plan could not be updated");
    vi.spyOn(console, "error").mockImplementation(() => {});
    const controller = await renderHook();

    await act(async () => {
      await controller().addPaths(["/a/report.pdf"]);
    });

    expect(useUiStore.getState().sourceNotice).toBe("the plan could not be updated");
    expect(useUiStore.getState().isIngesting).toBe(false);
  });

  it("ignores a second add while one is already in flight", async () => {
    useUiStore.setState({ isIngesting: true });
    const controller = await renderHook();

    await act(async () => {
      await controller().addPaths(["/a/report.pdf"]);
    });

    expect(expandPaths).not.toHaveBeenCalled();
  });

  it("raises the in-flight flag for as long as the add runs, and turns a second one away", async () => {
    let release!: (expanded: string[]) => void;
    expandPaths.mockReturnValue(
      new Promise<string[]>((resolve) => {
        release = resolve;
      }),
    );
    const controller = await renderHook();

    let firstAdd!: Promise<void>;
    await act(async () => {
      firstAdd = controller().addPaths(["/a/report.pdf"]);
    });

    expect(useUiStore.getState().isIngesting).toBe(true);
    expect(controller().isIngesting).toBe(true);

    await act(async () => {
      await controller().addPaths(["/b/other.pdf"]);
    });

    expect(expandPaths).toHaveBeenCalledTimes(1);

    await act(async () => {
      release(["/a/report.pdf"]);
      await firstAdd;
    });

    expect(useUiStore.getState().isIngesting).toBe(false);
    expect(controller().isIngesting).toBe(false);
    expect(addSources).toHaveBeenCalledTimes(1);
    expect(addSources).toHaveBeenCalledWith(["/a/report.pdf"]);
  });
});
