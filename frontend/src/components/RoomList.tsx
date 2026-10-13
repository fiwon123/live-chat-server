import { useMemo, useState, type FormEvent, type RefObject } from "react";

import type { Room } from "../api/types";
import { VISIBLE_ROOMS, selectRooms } from "../chat/rooms";
import { Icon } from "./Icon";

export interface RoomListProps {
  rooms: Room[];
  activeRoomId: string | null;
  /** Current user's id; rooms they created are the only deletable ones. */
  currentUserId: string;
  loading: boolean;
  error: string | null;
  unread: Record<string, number>;
  searchRef?: RefObject<HTMLInputElement | null>;
  onEnter: (room: Room) => void;
  onCreate: (name: string) => Promise<void>;
  onDelete: (room: Room) => Promise<void>;
}

export function RoomList({
  rooms,
  activeRoomId,
  currentUserId,
  loading,
  error,
  unread,
  searchRef,
  onEnter,
  onCreate,
  onDelete,
}: RoomListProps) {
  const [name, setName] = useState("");
  const [filter, setFilter] = useState("");
  const [busy, setBusy] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const [deleting, setDeleting] = useState<string | null>(null);

  const query = filter.trim().toLowerCase();

  const { visible, hiddenCount, total } = useMemo(
    () => selectRooms(rooms, unread, query, expanded),
    [rooms, unread, query, expanded],
  );

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

  const confirmDelete = async (room: Room) => {
    const ok = window.confirm(
      `Delete #${room.name}? Its messages are removed for everyone and this cannot be undone.`,
    );
    if (!ok) {
      return;
    }
    setDeleting(room.id);
    try {
      await onDelete(room);
    } finally {
      setDeleting(null);
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

      {!loading && rooms.length > 0 && total === 0 && (
        <p className="muted rooms__none">No rooms match “{filter.trim()}”.</p>
      )}

      <ul className="rooms__list">
        {visible.map((room) => {
          const count = unread[room.id] ?? 0;
          const owned = room.created_by === currentUserId;
          return (
            <li key={room.id} className="rooms__row">
              <button
                type="button"
                className={`rooms__enter${room.id === activeRoomId ? " active" : ""}`}
                onClick={() => onEnter(room)}
              >
                <Icon name="hash" size={16} className="rooms__hash" />
                <span className="rooms__main">
                  <span className="rooms__name">{room.name}</span>
                  {/* `!= null` (not `!== null`) on purpose: a version-skewed
                      backend (older than the `last_message` field, #45) omits
                      the key entirely, yielding `undefined`. */}
                  {room.last_message != null && (
                    <span className="rooms__preview" aria-hidden="true">
                      {room.last_message.username}: {room.last_message.body}
                    </span>
                  )}
                </span>
                {count > 0 && (
                  <span className="unread-badge" aria-hidden="true">
                    {count > 99 ? "99+" : count}
                  </span>
                )}
                {!room.member && <span className="tag">join</span>}
                {count > 0 && <span className="visually-hidden">{`${count} unread`}</span>}
              </button>
              {owned && (
                <button
                  type="button"
                  className="rooms__delete"
                  onClick={() => void confirmDelete(room)}
                  disabled={deleting === room.id}
                  aria-label={`Delete room ${room.name}`}
                  title="Delete room"
                >
                  <Icon name="trash" size={15} />
                </button>
              )}
            </li>
          );
        })}
      </ul>

      {total > VISIBLE_ROOMS && (
        <button
          type="button"
          className="rooms__more"
          aria-expanded={expanded}
          onClick={() => setExpanded((value) => !value)}
        >
          {expanded ? "Show fewer" : `Show ${hiddenCount} more`}
        </button>
      )}
    </aside>
  );
}
