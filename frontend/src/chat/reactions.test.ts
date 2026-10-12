import { describe, expect, it } from "vitest";

import type { Message } from "../api/types";
import { applyReaction, hasReacted } from "./reactions";

function message(overrides: Partial<Message> & { id: string }): Message {
  return {
    room_id: "room",
    user_id: "author",
    username: "alice",
    body: "hi",
    created_at: "2026-01-01T00:00:00Z",
    reactions: [],
    ...overrides,
  };
}

describe("hasReacted", () => {
  it("detects a user's reaction", () => {
    expect(hasReacted({ emoji: "👍", count: 1, user_ids: ["u1"] }, "u1")).toBe(true);
    expect(hasReacted({ emoji: "👍", count: 1, user_ids: ["u1"] }, "u2")).toBe(false);
  });
});

describe("applyReaction", () => {
  it("adds a new group when a user reacts first", () => {
    const messages = [message({ id: "m1" })];
    const next = applyReaction(messages, "m1", "👍", "u1", true);
    expect(next[0].reactions).toEqual([{ emoji: "👍", count: 1, user_ids: ["u1"] }]);
  });

  it("increments the count when a second user adds the same emoji", () => {
    const messages = [
      message({ id: "m1", reactions: [{ emoji: "👍", count: 1, user_ids: ["u1"] }] }),
    ];
    const next = applyReaction(messages, "m1", "👍", "u2", true);
    expect(next[0].reactions).toEqual([{ emoji: "👍", count: 2, user_ids: ["u1", "u2"] }]);
  });

  it("removes a user's reaction and drops an emptied group", () => {
    const messages = [
      message({ id: "m1", reactions: [{ emoji: "👍", count: 1, user_ids: ["u1"] }] }),
    ];
    const next = applyReaction(messages, "m1", "👍", "u1", false);
    expect(next[0].reactions).toEqual([]);
  });

  it("keeps a group after one of two users removes their reaction", () => {
    const messages = [
      message({ id: "m1", reactions: [{ emoji: "👍", count: 2, user_ids: ["u1", "u2"] }] }),
    ];
    const next = applyReaction(messages, "m1", "👍", "u1", false);
    expect(next[0].reactions).toEqual([{ emoji: "👍", count: 1, user_ids: ["u2"] }]);
  });

  it("only touches the target message", () => {
    const messages = [message({ id: "m1" }), message({ id: "m2" })];
    const next = applyReaction(messages, "m2", "🎉", "u1", true);
    expect(next[0]).toBe(messages[0]);
    expect(next[1].reactions).toEqual([{ emoji: "🎉", count: 1, user_ids: ["u1"] }]);
  });

  it("returns the same array when the message is unknown", () => {
    const messages = [message({ id: "m1" })];
    expect(applyReaction(messages, "missing", "👍", "u1", true)).toBe(messages);
  });

  it("returns the same array when removing a reaction that does not exist", () => {
    const messages = [message({ id: "m1" })];
    expect(applyReaction(messages, "m1", "👍", "u1", false)).toBe(messages);
  });

  it("does not mutate the input list", () => {
    const messages = [message({ id: "m1" })];
    applyReaction(messages, "m1", "👍", "u1", true);
    expect(messages[0].reactions).toEqual([]);
  });
});
