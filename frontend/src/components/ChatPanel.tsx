import { Fragment, useEffect, useRef, useState, type FormEvent, type KeyboardEvent } from "react";

import type { Message, PresenceUser, Room } from "../api/types";
import { formatDayLabel, formatMessageTime, isSameDay, shouldGroup } from "../chat/messages";
import type { ConnectionStatus } from "../hooks/useChatRoom";
import { Avatar } from "./Avatar";
import { Icon } from "./Icon";

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
  const [showScrollButton, setShowScrollButton] = useState(false);

  const feedRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const pinnedRef = useRef(true);
  const loadingOlderRef = useRef(false);
  const prevHeightRef = useRef(0);

  const live = status === "open";

  // Follow the conversation when pinned; keep position when loading older pages.
  useEffect(() => {
    const feed = feedRef.current;
    if (!feed) {
      return;
    }
    if (loadingOlderRef.current) {
      loadingOlderRef.current = false;
      feed.scrollTop = feed.scrollHeight - prevHeightRef.current;
      return;
    }
    if (pinnedRef.current) {
      feed.scrollTop = feed.scrollHeight;
    }
  }, [messages, room.id]);

  // On (re)connect, anchor to the newest message.
  useEffect(() => {
    if (live) {
      pinnedRef.current = true;
      const feed = feedRef.current;
      if (feed) {
        feed.scrollTop = feed.scrollHeight;
      }
      setShowScrollButton(false);
    }
  }, [live, room.id]);

  const handleScroll = () => {
    const feed = feedRef.current;
    if (!feed) {
      return;
    }
    const distance = feed.scrollHeight - feed.scrollTop - feed.clientHeight;
    const pinned = distance < 64;
    pinnedRef.current = pinned;
    setShowScrollButton(!pinned);
  };

  const scrollToBottom = () => {
    const feed = feedRef.current;
    if (!feed) {
      return;
    }
    feed.scrollTo({
      top: feed.scrollHeight,
      behavior: pinnedRef.current ? "auto" : "smooth",
    });
    pinnedRef.current = true;
    setShowScrollButton(false);
  };

  const loadOlderClicked = () => {
    const feed = feedRef.current;
    if (feed) {
      prevHeightRef.current = feed.scrollHeight;
      loadingOlderRef.current = true;
    }
    onLoadOlder();
  };

  const resize = () => {
    const textarea = textareaRef.current;
    if (!textarea) {
      return;
    }
    textarea.style.height = "auto";
    textarea.style.height = `${Math.min(textarea.scrollHeight, 160)}px`;
  };

  const submit = () => {
    const body = draft.trim();
    if (body.length === 0 || !live) {
      return;
    }
    if (onSend(body)) {
      setDraft("");
      if (textareaRef.current) {
        textareaRef.current.style.height = "auto";
      }
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

  const shownOnline = online.slice(0, 3);
  const onlineLabel = online.length === 0 ? "0 online" : `${online.length} online`;
  const onlineTitle = online.map((user) => user.username).join(", ");

  return (
    <section className="chat">
      <header>
        <div className="chat__title">
          <Icon name="hash" size={18} className="chat__hash" />
          <h2>{room.name}</h2>
          <span className={`status status-${status}`}>{STATUS_LABEL[status]}</span>
        </div>

        <div className="online" title={`${onlineLabel}: ${onlineTitle}`}>
          <span className="online__avs">
            {shownOnline.map((user) => (
              <Avatar key={user.id} name={user.username} size={22} />
            ))}
          </span>
          {onlineLabel}
        </div>

        <button
          type="button"
          className="btn btn--ghost btn--icon"
          onClick={onLeave}
          aria-label={`Leave ${room.name}`}
          title="Leave room"
        >
          <Icon name="logout" size={18} />
        </button>
      </header>

      <div className="feed" ref={feedRef} onScroll={handleScroll}>
        {hasMore && (
          <button
            type="button"
            className="older"
            onClick={loadOlderClicked}
            disabled={loadingOlder}
          >
            {loadingOlder ? "Loading…" : "Load earlier messages"}
          </button>
        )}

        {messages.length === 0 && (
          <div className="chat__empty">
            <Icon name="message" size={22} />
            <p className="muted">No messages yet — say hello.</p>
          </div>
        )}

        {messages.map((message, index) => {
          const previous = index > 0 ? messages[index - 1] : undefined;
          const own = message.user_id === currentUserId;
          const newDay = previous === undefined || !isSameDay(previous, message);
          const grouped = previous !== undefined && shouldGroup(previous, message);
          return (
            <Fragment key={message.id}>
              {newDay && (
                <div className="divider" role="separator">
                  <span>{formatDayLabel(message.created_at)}</span>
                </div>
              )}
              <article className={`message${own ? " own" : ""}${grouped ? " grouped" : ""}`}>
                {!own && !grouped && <Avatar name={message.username} size={34} />}
                <div className="message__body">
                  <div className="bubble">
                    {!own && !grouped && <span className="author">{message.username}</span>}
                    <p>{message.body}</p>
                  </div>
                  <time className="message__time" dateTime={message.created_at}>
                    {formatMessageTime(message.created_at)}
                  </time>
                </div>
              </article>
            </Fragment>
          );
        })}
      </div>

      {showScrollButton && messages.length > 0 && (
        <button
          type="button"
          className="chat__jump"
          onClick={scrollToBottom}
          aria-label="Scroll to newest messages"
          title="Jump to newest"
        >
          <Icon name="arrow-down" size={18} />
        </button>
      )}

      {error !== null && (
        <p className="error error--banner chat__error" role="alert">
          <Icon name="alert" size={15} />
          {error}
        </p>
      )}

      <form className="composer" onSubmit={onSubmit}>
        <textarea
          ref={textareaRef}
          value={draft}
          onChange={(event) => {
            setDraft(event.target.value);
            resize();
          }}
          onKeyDown={onKeyDown}
          placeholder={
            live
              ? "Message… (Enter to send, Shift+Enter for a new line)"
              : "Waiting for connection…"
          }
          maxLength={4000}
          rows={1}
          disabled={!live}
          aria-label="Message"
        />
        <button
          type="submit"
          className="btn btn--primary composer__send"
          disabled={!live || draft.trim().length === 0}
          aria-label="Send message"
          title="Send"
        >
          <Icon name="send" size={18} />
        </button>
      </form>
    </section>
  );
}
