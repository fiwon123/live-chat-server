import { useMemo, useState, type FormEvent, type RefObject } from "react";

import type { Room } from "../api/types";
import { Icon } from "./Icon";

export interface RoomListProps {
  rooms: Room[];
  activeRoomId: string | null;
  loading: boolean;
  error: string | null;
  unread: Record<string, number>;
  searchRef?: RefObject<HTMLInputElement | null>;
  onEnter: (room: Room) => void;
  onCreate: (name: string) => Promise<void>;
}

export function RoomList({
  rooms,
  activeRoomId,
  loading,
  error,
  unread,
  searchRef,
  onEnter,
  onCreate,
}: RoomListProps) {
  const [name, setName] = useState("");
  const [filter, setFilter] = useState("");
  const [busy, setBusy] = useState(false);

  const query = filter.trim().toLowerCase();
  const filtered = useMemo(() => {
    if (query.length === 0) {
      return rooms;
    }
    return rooms.filter((room) => room.name.toLowerCase().includes(query));
  }, [rooms, query]);

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    const trimmed = name.trim();
    if (trimmed.length === 0) {
      return;
    }
    setBusy(true);
    try {
      await onCreate(trimmed);
      setName("");
    } finally {
      setBusy(false);
    }
  };

  return (
    <aside className="rooms">
      <header className="rooms__head">
        <h2>Rooms</h2>
        <span className="rooms__count">{rooms.length}</span>
      </header>

      <form className="rooms__create" onSubmit={submit}>
        <input
          value={name}
          onChange={(event) => setName(event.target.value)}
          placeholder="New room name"
          maxLength={64}
          aria-label="New room name"
        />
        <button
          type="submit"
          className="btn btn--primary btn--icon"
          disabled={busy || name.trim().length === 0}
          aria-label="Create room"
          title="Create room"
        >
          <Icon name="plus" size={18} />
        </button>
      </form>

      <div className="rooms__search">
        <Icon name="search" size={15} className="rooms__search-icon" />
        <input
          ref={searchRef}
          value={filter}
          onChange={(event) => setFilter(event.target.value)}
          placeholder="Filter rooms…"
          aria-label="Filter rooms"
        />
        {filter.length > 0 && (
          <button
            type="button"
            className="rooms__search-clear"
            onClick={() => setFilter("")}
            aria-label="Clear filter"
          >
            <Icon name="close" size={14} />
          </button>
        )}
      </div>

      {error !== null && <p className="error">{error}</p>}

      {loading && rooms.length === 0 && (
        <div className="rooms__skeleton" aria-hidden="true">
          <span />
          <span />
          <span />
        </div>
      )}

      {!loading && rooms.length === 0 && (
        <div className="rooms__empty">
          <Icon name="hash" size={22} />
          <p className="muted">
            No rooms yet —
            <br />
            create the first one.
          </p>
        </div>
      )}

      {!loading && rooms.length > 0 && filtered.length === 0 && (
        <p className="muted rooms__none">No rooms match “{query}”.</p>
      )}

      <ul className="rooms__list">
        {filtered.map((room) => {
          const count = unread[room.id] ?? 0;
          return (
            <li key={room.id}>
              <button
                type="button"
                className={room.id === activeRoomId ? "active" : ""}
                onClick={() => onEnter(room)}
              >
                <Icon name="hash" size={16} className="rooms__hash" />
                <span className="rooms__name">{room.name}</span>
                {count > 0 && (
                  <span className="unread-badge" aria-label={`${count} unread`}>
                    {count > 99 ? "99+" : count}
                  </span>
                )}
                {!room.member && <span className="tag">join</span>}
              </button>
            </li>
          );
        })}
      </ul>
    </aside>
  );
}
