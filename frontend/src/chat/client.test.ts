import { describe, expect, it } from "vitest";

import {
  addUnread,
  loadUnread,
  markRoomRead,
  saveUnread,
  totalUnread,
  type UnreadMap,
} from "./unread";
import { EMOJIS } from "./emojis";
import { hasAnyLink, linkified } from "./linkify";

describe("unread map", () => {
  it("increments per room", () => {
    let map: UnreadMap = {};
    map = addUnread(map, "a");
    map = addUnread(map, "a");
    map = addUnread(map, "b");
    expect(map).toEqual({ a: 2, b: 1 });
  });

  it("marks a room read without touching others", () => {
    const map = markRoomRead({ a: 3, b: 1 }, "a");
    expect(map).toEqual({ b: 1 });
  });

  it("does not mutate the input map", () => {
    const before: UnreadMap = { a: 1 };
    addUnread(before, "a");
    markRoomRead(before, "a");
    expect(before).toEqual({ a: 1 });
  });

  it("sums totals", () => {
    expect(totalUnread({ a: 2, b: 5 })).toBe(7);
    expect(totalUnread({})).toBe(0);
  });

  it("round-trips through storage", () => {
    const store = new Map<string, string>();
    const storage: Pick<Storage, "getItem" | "setItem"> = {
      getItem: (key) => store.get(key) ?? null,
      setItem: (key, value) => void store.set(key, value),
    };
    saveUnread(storage, { a: 4 });
    expect(loadUnread(storage)).toEqual({ a: 4 });
  });

  it("tolerates corrupted storage", () => {
    const storage = { getItem: () => "{not-json", setItem: () => {} };
    expect(loadUnread(storage)).toEqual({});
  });

  it("ignores non-positive and non-numeric entries", () => {
    const storage = {
      getItem: () => JSON.stringify({ a: -1, b: "x", c: 3 }),
      setItem: () => {},
    };
    expect(loadUnread(storage)).toEqual({ c: 3 });
  });
});

describe("linkified", () => {
  it("splits text around a URL", () => {
    expect(linkified("see https://example.com/x now")).toEqual([
      { text: "see ", href: null },
      { text: "https://example.com/x", href: "https://example.com/x" },
      { text: " now", href: null },
    ]);
  });

  it("handles multiple URLs", () => {
    const segments = linkified("a https://a.com b https://b.com c");
    expect(segments.filter((segment) => segment.href !== null)).toHaveLength(2);
  });

  it("returns plain text unchanged", () => {
    expect(linkified("hello world")).toEqual([{ text: "hello world", href: null }]);
  });

  it("does not treat bare domains as links", () => {
    expect(linkified("go to example.com")).toEqual([{ text: "go to example.com", href: null }]);
  });

  it("strips trailing punctuation from links", () => {
    const segments = linkified("done https://a.com.");
    expect(segments[1]?.href).toBe("https://a.com");
    expect(segments[2]?.text).toBe(".");
  });

  it("hasAnyLink short-circuits correctly", () => {
    expect(hasAnyLink("no links")).toBe(false);
    expect(hasAnyLink("try https://a.com")).toBe(true);
  });
});

describe("emoji set", () => {
  it("is non-empty and unique", () => {
    expect(EMOJIS.length).toBeGreaterThan(0);
    const emojis = EMOJIS.map((entry) => entry.emoji);
    expect(new Set(emojis).size).toBe(emojis.length);
  });
});
