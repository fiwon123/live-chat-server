import type { Room } from "../api/types";

/// Show at most this many rooms before the "show more" expander.
export const VISIBLE_ROOMS = 5;

/// Order rooms the user cares about first: their own memberships, then by
/// unread count, then most recent. The API returns rooms oldest-first, which
/// buries anything useful when the table is large.
export function orderRooms(rooms: Room[], unread: Record<string, number>): Room[] {
  return [...rooms].sort((a, b) => {
    if (a.member !== b.member) {
      return a.member ? -1 : 1;
    }
    const unreadDelta = (unread[b.id] ?? 0) - (unread[a.id] ?? 0);
    if (unreadDelta !== 0) {
      return unreadDelta;
    }
    return (b.created_at ?? "").localeCompare(a.created_at ?? "");
  });
}

export function filterRooms(rooms: Room[], query: string): Room[] {
  const needle = query.trim().toLowerCase();
  if (needle.length === 0) {
    return rooms;
  }
  return rooms.filter((room) => room.name.toLowerCase().includes(needle));
}

export interface RoomSelection {
  visible: Room[];
  hiddenCount: number;
  total: number;
}

/// Order, filter, then bound the list. `expanded` reveals every match.
export function selectRooms(
  rooms: Room[],
  unread: Record<string, number>,
  query: string,
  expanded: boolean,
): RoomSelection {
  const filtered = filterRooms(orderRooms(rooms, unread), query);
  const visible = expanded ? filtered : filtered.slice(0, VISIBLE_ROOMS);
  return { visible, hiddenCount: filtered.length - visible.length, total: filtered.length };
}
