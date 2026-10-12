import { useCallback, useEffect, useRef, useState } from "react";

import { api } from "./api/endpoints";
import { messageOf } from "./api/client";
import type { Room } from "./api/types";
import { notifyNewMessage, requestNotificationPermission } from "./chat/notify";
import { playChime, unlockAudio } from "./chat/sound";
import { AuthScreen } from "./components/AuthScreen";
import { Avatar } from "./components/Avatar";
import { ChatPanel } from "./components/ChatPanel";
import { Icon } from "./components/Icon";
import { Logo } from "./components/Logo";
import { RoomList } from "./components/RoomList";
import { ToastHost } from "./components/ToastHost";
import { useAuth } from "./hooks/useAuth";
import { useChatRoom } from "./hooks/useChatRoom";
import { useTheme } from "./hooks/useTheme";
import { useToasts } from "./hooks/useToasts";
import { useUnread } from "./hooks/useUnread";
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

  const { unread, total, bump, markRead, sync, clearAll } = useUnread();
  const { theme, toggle: toggleTheme } = useTheme();
  const { toasts, push, dismiss } = useToasts();

  const token = auth.token ?? "";
  const chat = useChatRoom(token, activeRoom?.id ?? "");
  const roomSearchRef = useRef<HTMLInputElement>(null);
  const baselineRef = useRef<string | null>(null);
  const readTimerRef = useRef<number | null>(null);
  const readAtRef = useRef(0);
  const lastVisibleMessageRef = useRef<string | null>(null);

  /** Smallest gap between message-level read calls (mark-all is never gated). */
  const READ_THROTTLE_MS = 800;

  // Reflect unread messages in the tab title.
  useEffect(() => {
    document.title = total > 0 ? `(${total}) live-chat` : "live-chat";
  }, [total]);

  // Search rooms with Ctrl/Cmd+K.
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setSidebarOpen(true);
        roomSearchRef.current?.focus();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  // A (re)connect or room switch resets the "live" baseline so history never chimes.
  useEffect(() => {
    if (chat.status === "open") {
      baselineRef.current = null;
      unlockAudio();
    }
  }, [chat.status, activeRoom?.id]);

  // Clear the unread badge locally and persist the read cursor remotely.
  const markRoomReadRemote = useCallback(
    (roomId: string, messageId?: string) => {
      markRead(roomId);
      if (token.length === 0) {
        return;
      }
      if (messageId === undefined) {
        // Mark-everything is explicit (entering a room / tab focus): send now.
        if (readTimerRef.current !== null) {
          window.clearTimeout(readTimerRef.current);
          readTimerRef.current = null;
        }
        lastVisibleMessageRef.current = null;
        readAtRef.current = Date.now();
        void api.read(token, roomId, null).catch(() => {
          // Read state is best-effort; a failure only leaves the badge showing.
        });
        return;
      }
      // Message-level reads are throttled, with a trailing send so the cursor
      // always ends up at the newest visible message.
      lastVisibleMessageRef.current = messageId;
      const now = Date.now();
      const delay = readAtRef.current + READ_THROTTLE_MS - now;
      const flush = () => {
        readTimerRef.current = null;
        readAtRef.current = Date.now();
        const cursor = lastVisibleMessageRef.current;
        lastVisibleMessageRef.current = null;
        void api.read(token, roomId, cursor).catch(() => {});
      };
      if (delay <= 0) {
        flush();
      } else if (readTimerRef.current === null) {
        readTimerRef.current = window.setTimeout(flush, delay);
      }
    },
    [token, markRead],
  );

  // Chime + notify on genuinely new incoming messages.
  useEffect(() => {
    const last = chat.messages[chat.messages.length - 1];
    if (!last || chat.status !== "open" || activeRoom === null) {
      return;
    }
    if (baselineRef.current === null) {
      baselineRef.current = last.id;
      return;
    }
    if (baselineRef.current === last.id) {
      return;
    }
    baselineRef.current = last.id;
    if (last.user_id === auth.user?.id) {
      return;
    }
    if (document.visibilityState === "visible") {
      // Reading the room keeps the cursor at the newest visible message.
      markRoomReadRemote(activeRoom.id, last.id);
      return;
    }
    playChime();
    bump(activeRoom.id);
    notifyNewMessage(`#${activeRoom.name}`, `${last.username}: ${last.body}`);
  }, [chat.messages, chat.status, activeRoom, auth.user?.id, bump, markRoomReadRemote]);

  // Entering a room / returning to the tab clears its unread count.
  useEffect(() => {
    if (activeRoom !== null) {
      markRoomReadRemote(activeRoom.id);
    }
  }, [activeRoom, markRoomReadRemote]);

  useEffect(() => {
    const onVisibility = () => {
      if (document.visibilityState === "visible" && activeRoom !== null) {
        markRoomReadRemote(activeRoom.id);
      }
    };
    document.addEventListener("visibilitychange", onVisibility);
    return () => document.removeEventListener("visibilitychange", onVisibility);
  }, [activeRoom, markRoomReadRemote]);

  // Drop the trailing read timer on unmount.
  useEffect(
    () => () => {
      if (readTimerRef.current !== null) {
        window.clearTimeout(readTimerRef.current);
      }
    },
    [],
  );

  const refreshRooms = useCallback(async () => {
    if (token.length === 0) {
      return;
    }
    setRoomsLoading(true);
    setRoomsError(null);
    try {
      const rooms = await api.listRooms(token);
      setRooms(rooms);
      sync(rooms);
    } catch (error) {
      setRoomsError(messageOf(error));
    } finally {
      setRoomsLoading(false);
    }
  }, [token, sync]);

  // Load rooms once authenticated; clear everything on logout.
  useEffect(() => {
    if (auth.status === "authenticated") {
      void refreshRooms();
    } else if (auth.status === "anonymous") {
      setRooms([]);
      setActiveRoom(null);
      setRoomsError(null);
      setSidebarOpen(false);
      clearAll();
    }
  }, [auth.status, refreshRooms, clearAll]);

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
        push("info", `Joined #${room.name}`);
      }
      setActiveRoom({ ...room, member: true });
    },
    [token, refreshRooms, push],
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
        push("info", `Created #${room.name}`);
      } catch (error) {
        setRoomsError(messageOf(error));
        push("error", messageOf(error));
      }
    },
    [token, refreshRooms, push],
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
        onSubmit={(mode, username, password) => {
          unlockAudio();
          void requestNotificationPermission();
          void auth.authenticate(mode, username, password);
        }}
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
            onClick={toggleTheme}
            aria-label={theme === "dark" ? "Switch to light theme" : "Switch to dark theme"}
            title="Toggle theme"
          >
            <Icon name={theme === "dark" ? "sun" : "moon"} size={18} />
          </button>
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
          unread={unread}
          searchRef={roomSearchRef}
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
            typing={chat.typing}
            reads={chat.reads}
            status={chat.status}
            error={chat.error}
            hasMore={chat.hasMore}
            loadingOlder={chat.loadingOlder}
            onSend={chat.send}
            onTyping={chat.sendTyping}
            onReact={chat.react}
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

      <ToastHost toasts={toasts} onDismiss={dismiss} />
    </div>
  );
}

export default App;
