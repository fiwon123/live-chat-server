import { useEffect, useRef, useState, type FormEvent, type KeyboardEvent } from "react";

import type { ConnectionStatus } from "../hooks/useChatRoom";
import type { Message, PresenceUser, Room } from "../api/types";

export interface ChatPanelProps {
  room: Room;
  currentUserId: string;
  messages: Message[];
  online: PresenceUser[];
  status: ConnectionStatus;
  error: string | null;
  hasMore: boolean;
  loadingOlder: boolean;
  onSend: (body: string) => boolean;
  onLoadOlder: () => void;
  onLeave: () => void;
}

const STATUS_LABEL: Record<ConnectionStatus, string> = {
  idle: "offline",
  connecting: "connecting…",
  open: "live",
  closed: "reconnecting…",
  error: "connection problem",
};

function formatTime(iso: string): string {
  const date = new Date(iso);
  return Number.isNaN(date.getTime())
    ? ""
    : date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

export function ChatPanel({
  room,
  currentUserId,
  messages,
  online,
  status,
  error,
  hasMore,
  loadingOlder,
  onSend,
  onLoadOlder,
  onLeave,
}: ChatPanelProps) {
  const [draft, setDraft] = useState("");
  const feedRef = useRef<HTMLDivElement>(null);

  // Follow the conversation as it grows.
  useEffect(() => {
    const feed = feedRef.current;
    if (feed) {
      feed.scrollTop = feed.scrollHeight;
    }
  }, [messages]);

  const submit = () => {
    const body = draft.trim();
    if (body.length === 0) {
      return;
    }
    if (onSend(body)) {
      setDraft("");
    }
  };

  const onSubmit = (event: FormEvent) => {
    event.preventDefault();
    submit();
  };

  const onKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      submit();
    }
  };

  const live = status === "open";

  return (
    <section className="chat">
      <header>
        <div>
          <h2>{room.name}</h2>
          <span className={`status status-${status}`}>{STATUS_LABEL[status]}</span>
        </div>
        <div className="online" title={online.map((user) => user.username).join(", ")}>
          <span className="dot" /> {online.length} online
        </div>
        <button type="button" className="leave" onClick={onLeave}>
          Leave
        </button>
      </header>

      <div className="feed" ref={feedRef}>
        {hasMore && (
          <button type="button" className="older" onClick={onLoadOlder} disabled={loadingOlder}>
            {loadingOlder ? "Loading…" : "Load earlier messages"}
          </button>
        )}
        {messages.length === 0 && <p className="muted">No messages yet — say hello.</p>}
        {messages.map((message) => {
          const own = message.user_id === currentUserId;
          return (
            <article key={message.id} className={own ? "message own" : "message"}>
              <div className="meta">
                <span className="author">{own ? "you" : message.username}</span>
                <time dateTime={message.created_at}>{formatTime(message.created_at)}</time>
              </div>
              <p>{message.body}</p>
            </article>
          );
        })}
      </div>

      <form className="composer" onSubmit={onSubmit}>
        <textarea
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={onKeyDown}
          placeholder={
            live
              ? "Message… (Enter to send, Shift+Enter for a new line)"
              : "Waiting for connection…"
          }
          maxLength={4000}
          rows={2}
          disabled={!live}
        />
        <button type="submit" disabled={!live || draft.trim().length === 0}>
          Send
        </button>
      </form>

      {error !== null && <p className="error">{error}</p>}
    </section>
  );
}
