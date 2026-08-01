/** Degrees in one clockwise quarter turn. */
export const QUARTER_TURN_DEGREES = 90;

/** How far clockwise a slot is turned, in degrees. */
export function rotationDegrees(quarterTurns: number): number {
  return quarterTurns * QUARTER_TURN_DEGREES;
}
