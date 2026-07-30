import { beforeEach, describe, expect, it, vi } from "vitest";
import { defaultOutputDir, joinPath, parentDir, rememberOutputDir } from "../output-dir";

const { downloadDir } = vi.hoisted(() => ({
  downloadDir: vi.fn(() => Promise.resolve("/Users/me/Downloads")),
}));

vi.mock("@tauri-apps/api/path", () => ({ downloadDir }));

describe("output directory", () => {
  beforeEach(() => {
    localStorage.clear();
    downloadDir.mockClear();
  });

  it("remembers the last output directory", async () => {
    rememberOutputDir("/Users/me/Documents");
    expect(await defaultOutputDir()).toBe("/Users/me/Documents");
  });

  it("falls back to the OS downloads directory on first run", async () => {
    localStorage.clear();
    expect(await defaultOutputDir()).toBe("/Users/me/Downloads");
  });

  it("returns the parent directory for POSIX and Windows paths", () => {
    expect(parentDir("/a/b/c.pdf")).toBe("/a/b");
    expect(parentDir("C:\\a\\b\\c.pdf")).toBe("C:\\a\\b");
    expect(parentDir("c.pdf")).toBe("");
  });

  it("joins paths with the directory separator already in use", () => {
    expect(joinPath("/a/b", "merged.pdf")).toBe("/a/b/merged.pdf");
    expect(joinPath("C:\\a\\b", "merged.pdf")).toBe("C:\\a\\b\\merged.pdf");
  });

  it("does not double a trailing directory separator", () => {
    expect(joinPath("/a/b/", "merged.pdf")).toBe("/a/b/merged.pdf");
    expect(joinPath("C:\\a\\b\\", "merged.pdf")).toBe("C:\\a\\b\\merged.pdf");
  });

  it("falls back to downloads when reading local storage fails", async () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("storage unavailable");
    });

    await expect(defaultOutputDir()).resolves.toBe("/Users/me/Downloads");
  });

  it("does not propagate a local storage write failure", () => {
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("storage unavailable");
    });

    expect(() => rememberOutputDir("/Users/me/Documents")).not.toThrow();
  });
});
