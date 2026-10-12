import { describe, expect, it } from "vitest";

import { normalizeMessage, normalizeRoom } from "./normalize";

describe("normalizeRoom", () => {
  it("keeps a well-formed payload unchanged", () => {
    const room = {
      id: "r1",
      name: "general",
      created_by: "u1",
      created_at: "2026-01-01T00:00:00Z",
      member: true,
      unread_count: 3,
      last_message: {
        id: "m1",
        user_id: "u1",
        username: "alice",
        body: "hi",
        created_at: "2026-01-01T00:00:00Z",
      },
    };
    expect(normalizeRoom(room)).toEqual(room);
  });

  it("degrades a version-skewed payload (missing optional fields)", () => {
    // A backend older than #45 never emits `last_message`/may omit
    // `unread_count`; both must degrade instead of yielding `undefined`.
    const stale = {
      id: "r1",
      name: "general",
      created_by: "u1",
      created_at: "2026-01-01T00:00:00Z",
      member: true,
    };
    const room = normalizeRoom(stale);
    expect(room.last_message).toBeNull();
    expect(room.unread_count).toBe(0);
  });

  it("coerces invalid unread counts to 0", () => {
    const room = normalizeRoom({
      id: "r1",
      name: "general",
      created_by: "u1",
      created_at: "x",
      member: true,
      unread_count: Number.NaN,
    });
    expect(room.unread_count).toBe(0);
  });
});

describe("normalizeMessage", () => {
  it("defaults a missing reactions key to an empty array", () => {
    const message = normalizeMessage({
      id: "m1",
      room_id: "r1",
      user_id: "u1",
      username: "alice",
      body: "hi",
      created_at: "2026-01-01T00:00:00Z",
    });
    expect(message.reactions).toEqual([]);
  });

  it("keeps existing reactions", () => {
    const message = normalizeMessage({
      id: "m1",
      room_id: "r1",
      user_id: "u1",
      username: "alice",
      body: "hi",
      created_at: "2026-01-01T00:00:00Z",
      reactions: [{ emoji: "👍", count: 1, user_ids: ["u1"] }],
    });
    expect(message.reactions).toEqual([{ emoji: "👍", count: 1, user_ids: ["u1"] }]);
  });
});
