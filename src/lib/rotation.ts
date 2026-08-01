/** A slot's clockwise quarter-turn count, as the snapshot reports it. */
export const QUARTER_TURN_DEGREES = 90;

/** How far clockwise a slot is turned, in degrees. */
export function rotationDegrees(quarterTurns: number): number {
  return quarterTurns * QUARTER_TURN_DEGREES;
}
