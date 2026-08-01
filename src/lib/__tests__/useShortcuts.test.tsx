import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useUiStore } from "../../store/ui-store";
import { useShortcuts } from "../useShortcuts";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let root: Root | undefined;

function ShortcutClients({
  onRemove,
  onRotateLeft,
  onRotateRight,
  onUndo,
}: {
  onRemove: (event: KeyboardEvent) => boolean;
  onRotateLeft: (event: KeyboardEvent) => boolean;
  onRotateRight: (event: KeyboardEvent) => boolean;
  onUndo: (event: KeyboardEvent) => boolean;
}) {
  useShortcuts({ undo: onUndo });
  useShortcuts({ "remove-selected": onRemove });
  useShortcuts({ "rotate-left": onRotateLeft, "rotate-right": onRotateRight });
  return null;
}

async function renderClients(
  onUndo: (event: KeyboardEvent) => boolean,
  onRemove: (event: KeyboardEvent) => boolean,
  onRotateLeft: (event: KeyboardEvent) => boolean = () => false,
  onRotateRight: (event: KeyboardEvent) => boolean = () => false,
): Promise<void> {
  const container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
  await act(async () => {
    root?.render(
      <ShortcutClients
        onRemove={onRemove}
        onRotateLeft={onRotateLeft}
        onRotateRight={onRotateRight}
        onUndo={onUndo}
      />,
    );
  });
}

function shortcut(init: KeyboardEventInit): KeyboardEvent {
  return new KeyboardEvent("keydown", { bubbles: true, cancelable: true, ...init });
}

describe("useShortcuts", () => {
  beforeEach(() => {
    useUiStore.setState({ modalOpen: false });
  });

  afterEach(async () => {
    await act(async () => root?.unmount());
    root = undefined;
    document.body.replaceChildren();
    useUiStore.setState({ modalOpen: false });
    vi.restoreAllMocks();
  });

  it("adds only one keydown listener to window no matter how many components register", async () => {
    const addEventListener = vi.spyOn(window, "addEventListener");
    await renderClients(
      vi.fn(() => true),
      vi.fn(() => true),
    );

    expect(addEventListener.mock.calls.filter(([type]) => type === "keydown")).toHaveLength(1);
  });

  it("swaps in the current handler on re-render without reattaching the listener", async () => {
    const addEventListener = vi.spyOn(window, "addEventListener");
    const removeEventListener = vi.spyOn(window, "removeEventListener");
    const firstUndo = vi.fn(() => true);
    const currentUndo = vi.fn(() => true);
    const remove = vi.fn(() => false);
    const rotateLeft = vi.fn(() => false);
    const rotateRight = vi.fn(() => false);
    await renderClients(firstUndo, remove, rotateLeft, rotateRight);

    const initialUndo = shortcut({ ctrlKey: true, key: "z" });
    act(() => window.dispatchEvent(initialUndo));
    expect(firstUndo).toHaveBeenCalledOnce();
    expect(initialUndo.defaultPrevented).toBe(true);

    await act(async () => {
      root?.render(
        <ShortcutClients
          onRemove={remove}
          onRotateLeft={rotateLeft}
          onRotateRight={rotateRight}
          onUndo={currentUndo}
        />,
      );
    });
    const updatedUndo = shortcut({ ctrlKey: true, key: "z" });
    act(() => window.dispatchEvent(updatedUndo));

    expect(firstUndo).toHaveBeenCalledOnce();
    expect(currentUndo).toHaveBeenCalledOnce();
    expect(addEventListener.mock.calls.filter(([type]) => type === "keydown")).toHaveLength(1);
    expect(removeEventListener.mock.calls.filter(([type]) => type === "keydown")).toHaveLength(0);
  });

  it("leaves the event alone when a handler returns false", async () => {
    const remove = vi.fn(() => false);
    await renderClients(
      vi.fn(() => true),
      remove,
    );

    const declined = shortcut({ key: "Delete" });
    act(() => window.dispatchEvent(declined));

    expect(remove).toHaveBeenCalledOnce();
    expect(declined.defaultPrevented).toBe(false);
  });

  it("dispatches nothing while modalOpen is true", async () => {
    const undo = vi.fn(() => true);
    await renderClients(
      undo,
      vi.fn(() => true),
    );

    useUiStore.setState({ modalOpen: true });
    act(() => window.dispatchEvent(shortcut({ ctrlKey: true, key: "z" })));

    expect(undo).not.toHaveBeenCalled();
  });

  it("does not treat an event whose default is already prevented as a shortcut", async () => {
    const undo = vi.fn(() => true);
    await renderClients(
      undo,
      vi.fn(() => true),
    );

    const alreadyHandled = shortcut({ ctrlKey: true, key: "z" });
    alreadyHandled.preventDefault();
    act(() => window.dispatchEvent(alreadyHandled));

    expect(undo).not.toHaveBeenCalled();
  });

  it("dispatches modifier bracket shortcuts to their registered handlers", async () => {
    const rotateLeft = vi.fn(() => true);
    const rotateRight = vi.fn(() => true);
    await renderClients(
      vi.fn(() => true),
      vi.fn(() => true),
      rotateLeft,
      rotateRight,
    );

    const right = shortcut({ ctrlKey: true, key: "]" });
    act(() => window.dispatchEvent(right));

    const left = shortcut({ ctrlKey: true, key: "[" });
    act(() => window.dispatchEvent(left));

    expect(rotateRight).toHaveBeenCalledWith(right);
    expect(rotateLeft).toHaveBeenCalledWith(left);
    expect(right.defaultPrevented).toBe(true);
    expect(left.defaultPrevented).toBe(true);
  });
});
