import { describe, expect, it } from "vitest";

import type { Message, PresenceUser } from "../api/types";
import { applyRead, seenBy, type ReadMap } from "./reads";

const alice: PresenceUser = { id: "u-alice", username: "alice" };
const bob: PresenceUser = { id: "u-bob", username: "bob" };

function message(id: string): Message {
  return {
    id,
    room_id: "r",
    user_id: "u-alice",
    username: "alice",
    body: "hello",
    created_at: "2026-01-01T00:00:00Z",
    reactions: [],
  };
}

const messages = [message("m1"), message("m2"), message("m3")];

describe("applyRead", () => {
  it("records the reader's newest cursor", () => {
    const map = applyRead({}, "m2", bob);
    expect(map["u-bob"]).toEqual({ message_id: "m2", username: "bob" });
  });

  it("replaces an older cursor for the same reader", () => {
    const map = applyRead(applyRead({}, "m1", bob), "m3", bob);
    expect(Object.keys(map)).toEqual(["u-bob"]);
    expect(map["u-bob"]?.message_id).toBe("m3");
  });

  it("keeps different readers separate", () => {
    const map = applyRead(applyRead({}, "m1", alice), "m3", bob);
    expect(map["u-alice"]?.message_id).toBe("m1");
    expect(map["u-bob"]?.message_id).toBe("m3");
  });
});

describe("seenBy", () => {
  it("excludes the viewer's own cursor", () => {
    const reads: ReadMap = {
      "u-alice": { message_id: "m3", username: "alice" },
      "u-bob": { message_id: "m3", username: "bob" },
    };
    expect(seenBy(messages, reads, "m3", "u-alice")).toEqual([{ id: "u-bob", username: "bob" }]);
  });

  it("lists every reader whose cursor reached the target", () => {
    const reads: ReadMap = {
      "u-bob": { message_id: "m2", username: "bob" },
      "u-carol": { message_id: "m1", username: "carol" },
    };
    // bob's cursor is at m2 (same position) → seen; carol is stuck at m1.
    expect(seenBy(messages, reads, "m2", "u-own").map((user) => user.username)).toEqual(["bob"]);
  });

  it("counts readers past the target as seen", () => {
    const reads: ReadMap = { "u-bob": { message_id: "m3", username: "bob" } };
    expect(seenBy(messages, reads, "m2", "u-own").map((user) => user.username)).toEqual(["bob"]);
  });

  it("sorts readers by username", () => {
    const reads: ReadMap = {
      "u-bob": { message_id: "m3", username: "bob" },
      "u-carol": { message_id: "m3", username: "carol" },
      "u-anne": { message_id: "m3", username: "anne" },
    };
    expect(seenBy(messages, reads, "m3", "u-own").map((user) => user.username)).toEqual([
      "anne",
      "bob",
      "carol",
    ]);
  });

  it("ignores cursors whose message is not loaded", () => {
    const reads: ReadMap = { "u-bob": { message_id: "not-loaded", username: "bob" } };
    expect(seenBy(messages, reads, "m3", "u-own")).toEqual([]);
  });

  it("returns nobody when the target message is not loaded", () => {
    const reads: ReadMap = { "u-bob": { message_id: "m3", username: "bob" } };
    expect(seenBy(messages, reads, "m0", "u-own")).toEqual([]);
  });
});
