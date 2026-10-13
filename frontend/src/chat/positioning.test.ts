import { describe, expect, it } from "vitest";

import { REACTION_BAR_CLEARANCE, reactionBarOpensBelow } from "./positioning";

describe("reactionBarOpensBelow", () => {
  it("opens below when the message is close to the top of the feed", () => {
    expect(reactionBarOpensBelow(0)).toBe(true);
    expect(reactionBarOpensBelow(REACTION_BAR_CLEARANCE - 1)).toBe(true);
  });

  it("stays above once there is enough room", () => {
    expect(reactionBarOpensBelow(REACTION_BAR_CLEARANCE)).toBe(false);
    expect(reactionBarOpensBelow(400)).toBe(false);
  });

  it("honours a custom clearance", () => {
    expect(reactionBarOpensBelow(20, 10)).toBe(false);
    expect(reactionBarOpensBelow(5, 10)).toBe(true);
  });
});
