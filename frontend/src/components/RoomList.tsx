import { useState, type FormEvent } from "react";

import type { Room } from "../api/types";

export interface RoomListProps {
  rooms: Room[];
  activeRoomId: string | null;
  loading: boolean;
  error: string | null;
  onEnter: (room: Room) => void;
  onCreate: (name: string) => Promise<void>;
}

export function RoomList({
  rooms,
  activeRoomId,
  loading,
  error,
  onEnter,
  onCreate,
}: RoomListProps) {
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);

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
      <h2>Rooms</h2>

      <form onSubmit={submit}>
        <input
          value={name}
          onChange={(event) => setName(event.target.value)}
          placeholder="new room"
          maxLength={64}
          aria-label="New room name"
        />
        <button type="submit" disabled={busy || name.trim().length === 0}>
          Create
        </button>
      </form>

      {error !== null && <p className="error">{error}</p>}
      {loading && rooms.length === 0 && <p className="muted">Loading…</p>}

      <ul>
        {rooms.map((room) => (
          <li key={room.id}>
            <button
              type="button"
              className={room.id === activeRoomId ? "active" : ""}
              onClick={() => onEnter(room)}
            >
              <span className="name">{room.name}</span>
              {!room.member && <span className="tag">join</span>}
            </button>
          </li>
        ))}
      </ul>

      {rooms.length === 0 && !loading && <p className="muted">No rooms yet — create one.</p>}
    </aside>
  );
}
