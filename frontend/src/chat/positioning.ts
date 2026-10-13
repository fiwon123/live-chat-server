// Geometry for the per-message hover reaction bar.
//
// The bar floats fully above the bubble. When a message sits too close to the
// top of the scrolling feed, floating above would clip it, so the bar drops
// below the bubble instead. The decision is pure so it can be unit-tested.

/** Vertical room (px) a message needs above it to float the bar there. */
export const REACTION_BAR_CLEARANCE = 40;

/** Whether the reaction bar must open below the bubble for `spaceAbove`. */
export function reactionBarOpensBelow(
  spaceAbove: number,
  clearance = REACTION_BAR_CLEARANCE,
): boolean {
  return spaceAbove < clearance;
}
