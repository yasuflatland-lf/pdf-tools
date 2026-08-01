import { describe, expect, it } from "vitest";
import { QUARTER_TURN_DEGREES, rotationDegrees } from "../rotation";

describe("rotationDegrees", () => {
  it("defines a quarter turn as 90 degrees", () => {
    expect(QUARTER_TURN_DEGREES).toBe(90);
  });

  it.each([
    [0, 0],
    [1, 90],
    [2, 180],
    [3, 270],
  ])("converts %i quarter turns to %i degrees", (quarterTurns, degrees) => {
    expect(rotationDegrees(quarterTurns)).toBe(degrees);
  });
});
