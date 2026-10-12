import { useCallback, useEffect, useState } from "react";

import { api } from "./api/endpoints";
import { messageOf } from "./api/client";
import type { Room } from "./api/types";
import { AuthScreen } from "./components/AuthScreen";
import { Avatar } from "./components/Avatar";
import { ChatPanel } from "./components/ChatPanel";
import { Icon } from "./components/Icon";
import { Logo } from "./components/Logo";
import { RoomList } from "./components/RoomList";
import { useAuth } from "./hooks/useAuth";
import { useChatRoom } from "./hooks/useChatRoom";
import "./styles/primitives.css";
import "./styles/auth.css";
import "./styles/shell.css";
import "./styles/chat.css";

function App() {
  const auth = useAuth();
  const [rooms, setRooms] = useState<Room[]>([]);
  const [roomsLoading, setRoomsLoading] = useState(false);
  const [roomsError, setRoomsError] = useState<string | null>(null);
  const [activeRoom, setActiveRoom] = useState<Room | null>(null);
  const [sidebarOpen, setSidebarOpen] = useState(false);

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
      setSidebarOpen(false);
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
    return (
      <div className="splash" role="status">
        <Logo size={34} />
        <span className="visually-hidden">Loading…</span>
      </div>
    );
  }

  if (auth.status !== "authenticated" || auth.user === null) {
    return (
      <AuthScreen
        onSubmit={(mode, username, password) => void auth.authenticate(mode, username, password)}
        busy={auth.busy}
        error={auth.error}
      />
    );
  }

  return (
    <div className="app">
      <header className="topbar">
        <button
          type="button"
          className="btn btn--ghost btn--icon topbar__menu"
          onClick={() => setSidebarOpen((open) => !open)}
          aria-label="Toggle room list"
          aria-expanded={sidebarOpen}
        >
          <Icon name="menu" size={20} />
        </button>
        <Logo withWordmark size={24} />
        <div className="topbar__right">
          <Avatar name={auth.user.username} size={28} />
          <span className="who">{auth.user.username}</span>
          <button
            type="button"
            className="btn btn--ghost btn--icon"
            onClick={auth.logout}
            aria-label="Log out"
            title="Log out"
          >
            <Icon name="logout" size={18} />
          </button>
        </div>
      </header>

      <div className={`body${sidebarOpen ? " sidebar-open" : ""}`}>
        <RoomList
          rooms={rooms}
          activeRoomId={activeRoom?.id ?? null}
          loading={roomsLoading}
          error={roomsError}
          onEnter={(room) => {
            setSidebarOpen(false);
            void enterRoom(room);
          }}
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
            <div className="chat__empty">
              <Icon name="message" size={26} />
              <p className="muted">Select a room to start chatting.</p>
            </div>
          </section>
        )}
      </div>
    </div>
  );
}

export default App;
