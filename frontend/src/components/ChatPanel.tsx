import {
  Fragment,
  useEffect,
  useMemo,
  useRef,
  useState,
  type FormEvent,
  type KeyboardEvent,
} from "react";

import type { Message, PresenceUser, Room } from "../api/types";
import {
  formatDayLabel,
  formatMessageTime,
  isSameDay,
  shouldGroup,
  typingLabel,
} from "../chat/messages";
import { linkified } from "../chat/linkify";
import { QUICK_REACTIONS, hasReacted } from "../chat/reactions";
import { seenBy, type ReadMap } from "../chat/reads";
import type { ConnectionStatus } from "../hooks/useChatRoom";
import { Avatar } from "./Avatar";
import { EmojiPicker } from "./EmojiPicker";
import { Icon } from "./Icon";

export interface ChatPanelProps {
  room: Room;
  currentUserId: string;
  messages: Message[];
  online: PresenceUser[];
  typing: PresenceUser[];
  /** Read cursors per reader, surfaced as "Seen" on the viewer's last message. */
  reads: ReadMap;
  status: ConnectionStatus;
  error: string | null;
  hasMore: boolean;
  loadingOlder: boolean;
  onSend: (body: string) => boolean;
  onTyping: () => void;
  onReact: (messageId: string, emoji: string) => void;
  onLoadOlder: () => void;
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
  typing,
  reads,
  status,
  error,
  hasMore,
  loadingOlder,
  onSend,
  onTyping,
  onReact,
  onLoadOlder,
}: ChatPanelProps) {
  const [draft, setDraft] = useState("");
  const [showScrollButton, setShowScrollButton] = useState(false);
  // Id of the message whose reaction bar must open *below* the bubble because
  // there is not enough room above it inside the scrolling feed.
  const [actionsBelowId, setActionsBelowId] = useState<string | null>(null);

  const feedRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const pinnedRef = useRef(true);
  const loadingOlderRef = useRef(false);
  const prevHeightRef = useRef(0);

  const live = status === "open";

  // Room above a message needed to float its reaction bar without clipping.
  const ACTIONS_CLEARANCE = 40;

  /**
   * Decide whether the reaction bar floats above or below the bubble. Runs
   * when the bar is revealed (hover/focus) — never on selection — so picking
   * an emoji leaves the bar exactly where it is.
   */
  const positionActions = (messageId: string, element: HTMLElement) => {
    const feed = feedRef.current;
    if (!feed) {
      return;
    }
    const spaceAbove = element.getBoundingClientRect().top - feed.getBoundingClientRect().top;
    // Same value on repeat hovers bails out of the re-render in React.
    setActionsBelowId(spaceAbove < ACTIONS_CLEARANCE ? messageId : null);
  };

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

  // A failed/empty older-page load never changes `messages`, so the effect
  // above would leave loadingOlderRef stuck — clear it whenever the hook
  // reports the load finished.
  useEffect(() => {
    if (!loadingOlder) {
      loadingOlderRef.current = false;
    }
  }, [loadingOlder]);

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

  const insertEmoji = (emoji: string) => {
    const textarea = textareaRef.current;
    const start = textarea?.selectionStart ?? draft.length;
    const end = textarea?.selectionEnd ?? start;
    const next = `${draft.slice(0, start)}${emoji}${draft.slice(end)}`;
    setDraft(next);
    window.requestAnimationFrame(() => {
      const position = start + emoji.length;
      textarea?.focus();
      textarea?.setSelectionRange(position, position);
      resize();
    });
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
  const typingText = typingLabel(typing.filter((user) => user.id !== currentUserId));

  // "Seen" markers: the viewers whose read cursor reached the viewer's newest
  // message (their own cursor is not a receipt to themselves).
  const lastOwnMessageId = useMemo(() => {
    for (let index = messages.length - 1; index >= 0; index -= 1) {
      if (messages[index].user_id === currentUserId) {
        return messages[index].id;
      }
    }
    return null;
  }, [messages, currentUserId]);

  const seen = useMemo(
    () =>
      lastOwnMessageId === null ? [] : seenBy(messages, reads, lastOwnMessageId, currentUserId),
    [messages, reads, lastOwnMessageId, currentUserId],
  );

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
              <article
                className={`message${own ? " own" : ""}${grouped ? " grouped" : ""}`}
                onPointerEnter={(event) => positionActions(message.id, event.currentTarget)}
                onFocus={(event) => positionActions(message.id, event.currentTarget)}
              >
                {!own && !grouped && <Avatar name={message.username} size={34} />}
                <div className="message__body">
                  <div className="bubble">
                    {!own && !grouped && <span className="author">{message.username}</span>}
                    <p>
                      {linkified(message.body).map((segment, partIndex) =>
                        segment.href !== null ? (
                          <a
                            key={partIndex}
                            href={segment.href}
                            target="_blank"
                            rel="noopener noreferrer"
                          >
                            {segment.text}
                          </a>
                        ) : (
                          <Fragment key={partIndex}>{segment.text}</Fragment>
                        ),
                      )}
                    </p>
                  </div>
                  {message.reactions.length > 0 && (
                    <div className="reactions">
                      {message.reactions.map((group) => {
                        const mine = hasReacted(group, currentUserId);
                        return (
                          <button
                            key={group.emoji}
                            type="button"
                            className={`reaction${mine ? " reaction--mine" : ""}`}
                            onClick={() => onReact(message.id, group.emoji)}
                            disabled={!live || own}
                            aria-pressed={mine}
                            aria-label={`${group.emoji}, ${group.count} ${
                              group.count === 1 ? "reaction" : "reactions"
                            }`}
                            title={`${group.count} ${group.count === 1 ? "reaction" : "reactions"}`}
                          >
                            <span className="reaction__emoji" aria-hidden="true">
                              {group.emoji}
                            </span>
                            <span className="reaction__count">{group.count}</span>
                          </button>
                        );
                      })}
                    </div>
                  )}
                  <time className="message__time" dateTime={message.created_at}>
                    {formatMessageTime(message.created_at)}
                  </time>
                  {message.id === lastOwnMessageId && seen.length > 0 && (
                    <span
                      className="message__seen"
                      title={seen.map((user) => user.username).join(", ")}
                    >
                      Seen{seen.length === 1 ? ` by ${seen[0].username}` : ` by ${seen.length}`}
                    </span>
                  )}
                </div>
                {!own && (
                  <div
                    className={`message__actions${
                      actionsBelowId === message.id ? " message__actions--below" : ""
                    }`}
                    role="group"
                    aria-label="Add a reaction"
                  >
                    {QUICK_REACTIONS.map((emoji) => {
                      const active = message.reactions.some(
                        (group) => group.emoji === emoji && hasReacted(group, currentUserId),
                      );
                      return (
                        <button
                          key={emoji}
                          type="button"
                          className={`quick-reaction${active ? " quick-reaction--mine" : ""}`}
                          onClick={() => onReact(message.id, emoji)}
                          disabled={!live}
                          aria-pressed={active}
                          aria-label={`React with ${emoji}`}
                          title={`React with ${emoji}`}
                        >
                          {emoji}
                        </button>
                      );
                    })}
                  </div>
                )}
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

      <div className="typing" aria-live="polite" aria-atomic="true">
        {typingText !== null && (
          <>
            <span className="typing__dots" aria-hidden="true">
              <span />
              <span />
              <span />
            </span>
            {typingText}
          </>
        )}
      </div>

      {error !== null && (
        <p className="error error--banner chat__error" role="alert">
          <Icon name="alert" size={15} />
          {error}
        </p>
      )}

      <form className="composer" onSubmit={onSubmit}>
        <EmojiPicker onPick={insertEmoji} disabled={!live} />
        <textarea
          ref={textareaRef}
          value={draft}
          onChange={(event) => {
            setDraft(event.target.value);
            onTyping();
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
