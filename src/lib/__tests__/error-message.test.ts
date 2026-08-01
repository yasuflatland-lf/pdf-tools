import { describe, expect, it } from "vitest";
import { errorMessage } from "../error-message";

describe("errorMessage", () => {
  it("returns a string as itself", () => {
    expect(errorMessage("plain rejection")).toBe("plain rejection");
  });

  it("returns an Error's message", () => {
    expect(errorMessage(new Error("application failure"))).toBe("application failure");
  });

  it.each([null, undefined, 42, { reason: "unknown" }])(
    "stringifies any other value: %o",
    (value) => {
      expect(errorMessage(value)).toBe(String(value));
    },
  );
});
