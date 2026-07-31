import { describe, expect, it } from "vitest";
import { nextFocusIndex, resolveShortcut, shortcutHint, type ShortcutAction } from "../keyboard";

describe("resolveShortcut", () => {
  it("maps Delete and Backspace to removing the selection", () => {
    expect(resolveShortcut({ key: "Delete" })).toBe("remove-selected");
    expect(resolveShortcut({ key: "Backspace" })).toBe("remove-selected");
  });

  it("maps the platform modifier plus A to select-all", () => {
    expect(resolveShortcut({ key: "a", metaKey: true })).toBe("select-all");
    expect(resolveShortcut({ key: "a", ctrlKey: true })).toBe("select-all");
  });

  it("maps shift-modifier-Z to redo", () => {
    expect(resolveShortcut({ key: "z", metaKey: true, shiftKey: true })).toBe("redo");
    expect(resolveShortcut({ key: "z", metaKey: true })).toBe("undo");
  });

  it("ignores shortcuts while a text input has focus", () => {
    const inputElement = document.createElement("input");

    expect(resolveShortcut({ key: "Delete", target: inputElement })).toBeNull();
  });

  it("ignores shortcuts while a textarea or contenteditable element has focus", () => {
    const textarea = document.createElement("textarea");
    const editable = document.createElement("div");
    Object.defineProperty(editable, "isContentEditable", { value: true });

    expect(resolveShortcut({ key: "a", metaKey: true, target: textarea })).toBeNull();
    expect(resolveShortcut({ key: "Delete", target: editable })).toBeNull();
  });

  it("normalizes the uppercase Z reported by shifted keyboard chords", () => {
    expect(resolveShortcut({ key: "Z", ctrlKey: true, shiftKey: true })).toBe("redo");
  });

  it("maps Escape and all four arrow keys", () => {
    expect(resolveShortcut({ key: "Escape" })).toBe("clear-selection");
    expect(resolveShortcut({ key: "ArrowLeft" })).toBe("focus-previous");
    expect(resolveShortcut({ key: "ArrowRight" })).toBe("focus-next");
    expect(resolveShortcut({ key: "ArrowUp" })).toBe("focus-row-previous");
    expect(resolveShortcut({ key: "ArrowDown" })).toBe("focus-row-next");
  });

  it("ignores unmapped keys, modifier arrows, and alt chords", () => {
    expect(resolveShortcut({ key: "x" })).toBeNull();
    expect(resolveShortcut({ key: "ArrowRight", metaKey: true })).toBeNull();
    expect(resolveShortcut({ key: "Delete", altKey: true })).toBeNull();
  });
});

describe("nextFocusIndex", () => {
  it.each<[ShortcutAction, number, number, number, number | null]>([
    ["focus-previous", 2, 5, 2, 1],
    ["focus-next", 2, 5, 2, 3],
    ["focus-row-previous", 4, 6, 3, 1],
    ["focus-row-next", 1, 6, 3, 4],
    ["focus-previous", 0, 5, 2, 0],
    ["focus-next", 4, 5, 2, 4],
    ["focus-row-previous", 1, 5, 3, 0],
    ["focus-row-next", 3, 5, 3, 4],
    ["focus-next", -1, 5, 2, 0],
    ["focus-row-next", 10, 5, 2, 0],
    ["focus-previous", -1, 5, 2, 4],
    ["focus-row-previous", 10, 5, 2, 4],
    ["focus-row-next", 0, 3, 0, 1],
    ["select-all", 0, 5, 2, null],
    ["focus-next", 0, 0, 2, null],
  ])(
    "resolves %s from %i with count %i and %i columns",
    (action, currentIndex, count, columns, expected) => {
      expect(nextFocusIndex(action, currentIndex, count, columns)).toBe(expected);
    },
  );
});

describe("shortcutHint", () => {
  const mac = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15";
  const windows = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36";

  it("uses the macOS glyphs on a Mac", () => {
    expect(shortcutHint("undo", mac)).toBe("⌘Z");
    expect(shortcutHint("redo", mac)).toBe("⇧⌘Z");
  });

  it("spells the modifier out on every other platform", () => {
    expect(shortcutHint("undo", windows)).toBe("Ctrl+Z");
    expect(shortcutHint("redo", windows)).toBe("Ctrl+Shift+Z");
  });

  it("falls back to the spelled-out modifier when the user agent is unknown", () => {
    expect(shortcutHint("undo", "")).toBe("Ctrl+Z");
  });
});
