import { useCallback, useEffect, useState } from "react";

import { api } from "./api/endpoints";
import { messageOf } from "./api/client";
import type { Room } from "./api/types";
import { ChatPanel } from "./components/ChatPanel";
import { LoginForm } from "./components/LoginForm";
import { RoomList } from "./components/RoomList";
import { useAuth } from "./hooks/useAuth";
import { useChatRoom } from "./hooks/useChatRoom";
import "./App.css";

function App() {
  const auth = useAuth();
  const [rooms, setRooms] = useState<Room[]>([]);
  const [roomsLoading, setRoomsLoading] = useState(false);
  const [roomsError, setRoomsError] = useState<string | null>(null);
  const [activeRoom, setActiveRoom] = useState<Room | null>(null);

  const token = auth.token ?? "";
  const chat = useChatRoom(token, activeRoom?.id ?? "");

  const refreshRooms = useCallback(async () => {
    if (token.length === 0) {
      return;
    }
    setRoomsLoading(true);
    setRoomsError(null);
    try {
      setRooms(await api.listRooms(token));
    } catch (error) {
      setRoomsError(messageOf(error));
    } finally {
      setRoomsLoading(false);
    }
  }, [token]);

  // Load rooms once authenticated; clear everything on logout.
  useEffect(() => {
    if (auth.status === "authenticated") {
      void refreshRooms();
    } else if (auth.status === "anonymous") {
      setRooms([]);
      setActiveRoom(null);
      setRoomsError(null);
    }
  }, [auth.status, refreshRooms]);

  const enterRoom = useCallback(
    async (room: Room) => {
      if (!token) {
        return;
      }
      if (!room.member) {
        try {
          await api.joinRoom(token, room.id);
        } catch (error) {
          setRoomsError(messageOf(error));
          return;
        }
        void refreshRooms();
      }
      setActiveRoom({ ...room, member: true });
    },
    [token, refreshRooms],
  );

  const createRoom = useCallback(
    async (name: string) => {
      if (!token) {
        return;
      }
      try {
        const room = await api.createRoom(token, name);
        await refreshRooms();
        setActiveRoom(room);
      } catch (error) {
        setRoomsError(messageOf(error));
      }
    },
    [token, refreshRooms],
  );

  const leaveRoom = useCallback(async () => {
    const room = activeRoom;
    setActiveRoom(null);
    if (token && room) {
      try {
        await api.leaveRoom(token, room.id);
      } catch {
        // Leaving the view is enough; a failed API call shouldn't trap the user.
      }
      void refreshRooms();
    }
  }, [token, activeRoom, refreshRooms]);

  if (auth.status === "loading") {
    return <div className="splash">Loading…</div>;
  }

  if (auth.status !== "authenticated" || auth.user === null) {
    return (
      <main className="auth">
        <LoginForm
          onSubmit={(mode, username, password) => void auth.authenticate(mode, username, password)}
          busy={auth.busy}
          error={auth.error}
        />
      </main>
    );
  }

  return (
    <div className="app">
      <header className="topbar">
        <strong>live-chat</strong>
        <span className="who">{auth.user.username}</span>
        <button type="button" onClick={auth.logout}>
          Log out
        </button>
      </header>

      <div className="body">
        <RoomList
          rooms={rooms}
          activeRoomId={activeRoom?.id ?? null}
          loading={roomsLoading}
          error={roomsError}
          onEnter={(room) => void enterRoom(room)}
          onCreate={createRoom}
        />

        {activeRoom !== null ? (
          <ChatPanel
            room={activeRoom}
            currentUserId={auth.user.id}
            messages={chat.messages}
            online={chat.online}
            status={chat.status}
            error={chat.error}
            hasMore={chat.hasMore}
            loadingOlder={chat.loadingOlder}
            onSend={chat.send}
            onLoadOlder={chat.loadOlder}
            onLeave={() => void leaveRoom()}
          />
        ) : (
          <section className="chat empty">
            <p className="muted">Select a room to start chatting.</p>
          </section>
        )}
      </div>
    </div>
  );
}

export default App;
