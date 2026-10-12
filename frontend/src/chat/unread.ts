// Client-side per-room unread tracking, persisted to localStorage.

export type UnreadMap = Record<string, number>;

export const UNREAD_KEY = "livechat.unread.v1";

export function addUnread(map: UnreadMap, roomId: string): UnreadMap {
  return { ...map, [roomId]: (map[roomId] ?? 0) + 1 };
}

export function markRoomRead(map: UnreadMap, roomId: string): UnreadMap {
  if ((map[roomId] ?? 0) === 0) {
    return map;
  }
  const next = { ...map };
  delete next[roomId];
  return next;
}

/**
 * Reconcile the local map against the server's authoritative per-room counts.
 *
 * The server is the source of truth: a bump implies the server already
 * persisted the message (save-before-broadcast), so any fetch that resolves
 * after a bump reports a count of at least 1. Assigning the server count
 * (and dropping rooms the server reports as read) can therefore never lose a
 * live bump; it only retires stale ones.
 */
export function syncFromRooms(
  _map: UnreadMap,
  rooms: Array<{ id: string; unread_count: number }>,
): UnreadMap {
  const next: UnreadMap = {};
  for (const room of rooms) {
    const count = Math.max(0, Math.floor(room.unread_count));
    if (count > 0) {
      next[room.id] = count;
    }
  }
  return next;
}

export function totalUnread(map: UnreadMap): number {
  return Object.values(map).reduce((sum, count) => sum + count, 0);
}

export function loadUnread(storage: Pick<Storage, "getItem">): UnreadMap {
  try {
    const raw = storage.getItem(UNREAD_KEY);
    if (raw === null) {
      return {};
    }
    const parsed: unknown = JSON.parse(raw);
    if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
      return {};
    }
    const result: UnreadMap = {};
    for (const [key, value] of Object.entries(parsed)) {
      if (typeof key === "string" && typeof value === "number" && value > 0) {
        result[key] = Math.floor(value);
      }
    }
    return result;
  } catch {
    return {};
  }
}

export function saveUnread(storage: Pick<Storage, "setItem">, map: UnreadMap): void {
  try {
    storage.setItem(UNREAD_KEY, JSON.stringify(map));
  } catch {
    // Quota/private-mode failures are non-fatal.
  }
}
