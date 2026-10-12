// Per-room unread counts, backed by localStorage and kept in sync with the DOM.

import { useCallback, useEffect, useRef, useState } from "react";

import {
  addUnread,
  loadUnread,
  markRoomRead,
  saveUnread,
  syncFromRooms,
  totalUnread,
  type UnreadMap,
} from "../chat/unread";

export function useUnread() {
  const [unread, setUnread] = useState<UnreadMap>(() => loadUnread(window.localStorage));
  const first = useRef(true);

  useEffect(() => {
    if (first.current) {
      first.current = false;
      return;
    }
    saveUnread(window.localStorage, unread);
  }, [unread]);

  const bump = useCallback((roomId: string) => {
    setUnread((map) => addUnread(map, roomId));
  }, []);

  const markRead = useCallback((roomId: string) => {
    setUnread((map) => markRoomRead(map, roomId));
  }, []);

  /** Adopt the server's per-room counts (called after each room list fetch). */
  const sync = useCallback((rooms: Array<{ id: string; unread_count: number }>) => {
    setUnread((map) => syncFromRooms(map, rooms));
  }, []);

  const clearAll = useCallback(() => {
    setUnread({});
  }, []);

  return {
    unread,
    total: totalUnread(unread),
    bump,
    markRead,
    sync,
    clearAll,
  };
}
