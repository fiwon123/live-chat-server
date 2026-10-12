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
 * Server counts win where they are positive; a local bump is kept only when the
 * server reports zero for a room we already bumped (the list may predate the
 * bump — a message arrived while the tab was hidden). The next sync settles it.
 */
export function syncFromRooms(
  map: UnreadMap,
  rooms: Array<{ id: string; unread_count: number }>,
): UnreadMap {
  const next = { ...map };
  for (const room of rooms) {
    const count = Math.max(0, Math.floor(room.unread_count));
    if (count > 0) {
      next[room.id] = Math.max(next[room.id] ?? 0, count);
    } else if ((next[room.id] ?? 0) === 0) {
      delete next[room.id];
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
