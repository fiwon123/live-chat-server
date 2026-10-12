import { describe, expect, it } from "vitest";

import {
  addUnread,
  loadUnread,
  markRoomRead,
  saveUnread,
  syncFromRooms,
  totalUnread,
  type UnreadMap,
} from "./unread";
import { EMOJIS } from "./emojis";
import { linkified } from "./linkify";

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

describe("syncFromRooms", () => {
  it("adopts positive server counts over stale local ones", () => {
    const map = syncFromRooms({ a: 1, b: 9 }, [
      { id: "a", unread_count: 5 },
      { id: "b", unread_count: 0 },
    ]);
    expect(map.a).toBe(5);
  });

  it("keeps a local bump when the server list predates it", () => {
    const map = syncFromRooms({ b: 9 }, [
      { id: "a", unread_count: 0 },
      { id: "b", unread_count: 0 },
    ]);
    expect(map.b).toBe(9);
  });

  it("clears a room when both sides agree it is read", () => {
    const map = syncFromRooms({ a: 0 }, [{ id: "a", unread_count: 0 }]);
    expect(map.a).toBeUndefined();
  });

  it("ignores negative and fractional counts", () => {
    const map = syncFromRooms({}, [
      { id: "a", unread_count: -1 },
      { id: "b", unread_count: 2.7 },
    ]);
    expect(map).toEqual({ b: 2 });
  });

  it("does not mutate the input map", () => {
    const before: UnreadMap = { a: 1 };
    syncFromRooms(before, [{ id: "a", unread_count: 3 }]);
    expect(before).toEqual({ a: 1 });
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

  it("strips an unbalanced closing bracket", () => {
    const segments = linkified("(see https://a.com/x)");
    expect(segments[1]?.href).toBe("https://a.com/x");
    expect(segments[2]?.text).toBe(")");
  });

  it("keeps balanced brackets inside a URL", () => {
    const url = "https://en.wikipedia.org/wiki/Foo_(bar)";
    const segments = linkified(`read ${url}`);
    expect(segments[1]?.href).toBe(url);
  });
});

describe("emoji set", () => {
  it("is non-empty and unique", () => {
    expect(EMOJIS.length).toBeGreaterThan(0);
    const emojis = EMOJIS.map((entry) => entry.emoji);
    expect(new Set(emojis).size).toBe(emojis.length);
  });
});
