import { describe, expect, it } from "vitest";

import { VISIBLE_ROOMS, filterRooms, orderRooms, selectRooms } from "./rooms";
import type { Room } from "../api/types";

function room(overrides: Partial<Room> & { id: string }): Room {
  return {
    name: overrides.id,
    created_by: "owner",
    created_at: "2026-01-01T00:00:00.000Z",
    member: false,
    unread_count: 0,
    last_message: null,
    ...overrides,
  };
}

function ids(rooms: Room[]): string[] {
  return rooms.map((r) => r.id);
}

describe("orderRooms", () => {
  it("puts member rooms before non-member rooms", () => {
    const rooms = [room({ id: "a" }), room({ id: "b", member: true }), room({ id: "c" })];
    expect(ids(orderRooms(rooms, {}))).toEqual(["b", "a", "c"]);
  });

  it("sorts members by unread count, most unread first", () => {
    const rooms = [
      room({ id: "a", member: true }),
      room({ id: "b", member: true }),
      room({ id: "c", member: true }),
    ];
    const unread = { a: 1, b: 9, c: 4 };
    expect(ids(orderRooms(rooms, unread))).toEqual(["b", "c", "a"]);
  });

  it("breaks unread ties by most recent", () => {
    const rooms = [
      room({ id: "old", member: true, created_at: "2026-01-01T00:00:00.000Z" }),
      room({ id: "new", member: true, created_at: "2026-02-01T00:00:00.000Z" }),
    ];
    expect(ids(orderRooms(rooms, {}))).toEqual(["new", "old"]);
  });

  it("does not mutate the input array", () => {
    const rooms = [room({ id: "a" }), room({ id: "b", member: true })];
    orderRooms(rooms, {});
    expect(ids(rooms)).toEqual(["a", "b"]);
  });
});

describe("filterRooms", () => {
  it("matches case-insensitively on a substring", () => {
    const rooms = [room({ id: "General" }), room({ id: "random" })];
    expect(ids(filterRooms(rooms, "gen"))).toEqual(["General"]);
  });

  it("returns every room for an empty query", () => {
    const rooms = [room({ id: "a" }), room({ id: "b" })];
    expect(filterRooms(rooms, "  ")).toHaveLength(2);
  });
});

describe("selectRooms", () => {
  const many = Array.from({ length: VISIBLE_ROOMS + 3 }, (_, i) => room({ id: `r${i}` }));

  it("bounds the collapsed list and reports the hidden count", () => {
    const { visible, hiddenCount, total } = selectRooms(many, {}, "", false);
    expect(visible).toHaveLength(VISIBLE_ROOMS);
    expect(hiddenCount).toBe(3);
    expect(total).toBe(VISIBLE_ROOMS + 3);
  });

  it("reveals everything when expanded", () => {
    const { visible, hiddenCount } = selectRooms(many, {}, "", true);
    expect(visible).toHaveLength(VISIBLE_ROOMS + 3);
    expect(hiddenCount).toBe(0);
  });

  it("filters across the whole set before bounding", () => {
    const rooms = [
      ...Array.from({ length: VISIBLE_ROOMS }, (_, i) => room({ id: `other${i}` })),
      room({ id: "target" }),
    ];
    const { visible, hiddenCount, total } = selectRooms(rooms, {}, "target", false);
    expect(ids(visible)).toEqual(["target"]);
    expect(hiddenCount).toBe(0);
    expect(total).toBe(1);
  });
});
