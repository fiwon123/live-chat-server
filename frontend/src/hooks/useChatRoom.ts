// Live chat room: history fetch + WebSocket subscription for one room.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { messageOf } from "../api/client";
import { api } from "../api/endpoints";
import type { Message, PresenceUser, ServerEvent } from "../api/types";
import { encodeReact, encodeSend, encodeTyping, wsUrl } from "../api/ws";
import { mergeMessages } from "../chat/messages";
import { applyReaction } from "../chat/reactions";
import { applyRead, type ReadMap } from "../chat/reads";

export const HISTORY_PAGE_SIZE = 50;
const BASE_RECONNECT_DELAY_MS = 500;
const MAX_RECONNECT_DELAY_MS = 8000;
/** How long a received `typing` event keeps a user listed before it expires. */
export const TYPING_TTL_MS = 3500;
/** Client-side cap on how often we emit `typing` while the user types. */
export const TYPING_SEND_INTERVAL_MS = 1500;

export type ConnectionStatus = "idle" | "connecting" | "open" | "closed" | "error";

export interface ChatRoom {
  messages: Message[];
  online: PresenceUser[];
  typing: PresenceUser[];
  /** Read cursors per reader, updated by ephemeral `read` receipts. */
  reads: ReadMap;
  status: ConnectionStatus;
  error: string | null;
  hasMore: boolean;
  loadingOlder: boolean;
  send: (body: string) => boolean;
  sendTyping: () => void;
  react: (messageId: string, emoji: string) => void;
  loadOlder: () => void;
}

/**
 * Connect to a room's WebSocket and load its history. Reconnects with capped
 * exponential backoff; a room change tears the connection down.
 */
export function useChatRoom(token: string, roomId: string): ChatRoom {
  const [messages, setMessages] = useState<Message[]>([]);
  const [online, setOnline] = useState<Record<string, PresenceUser>>({});
  const [typing, setTyping] = useState<Record<string, PresenceUser>>({});
  const [reads, setReads] = useState<ReadMap>({});
  const [status, setStatus] = useState<ConnectionStatus>("idle");
  const [error, setError] = useState<string | null>(null);
  const [hasMore, setHasMore] = useState(false);
  const [loadingOlder, setLoadingOlder] = useState(false);

  const socketRef = useRef<WebSocket | null>(null);
  const oldestRef = useRef<string | null>(null);
  const typingTimersRef = useRef(new Map<string, number>());
  const typingSentAtRef = useRef(0);

  const active = token.length > 0 && roomId.length > 0;

  /** Drop every pending typing timer and clear the indicator. */
  const clearTyping = useCallback(() => {
    for (const timer of typingTimersRef.current.values()) {
      window.clearTimeout(timer);
    }
    typingTimersRef.current.clear();
    // A new room deserves an immediate typing frame, not the old room's gap.
    typingSentAtRef.current = 0;
    setTyping({});
  }, []);

  useEffect(() => {
    oldestRef.current = messages.length > 0 ? messages[0].id : null;
  }, [messages]);

  // History for the (new) room.
  useEffect(() => {
    setMessages([]);
    setOnline({});
    setReads({});
    clearTyping();
    setError(null);
    setHasMore(false);
    if (!active) {
      setStatus("idle");
      return;
    }
    let cancelled = false;
    api
      .history(token, roomId, { limit: HISTORY_PAGE_SIZE })
      .then((page) => {
        if (cancelled) {
          return;
        }
        setMessages(mergeMessages([], page));
        setHasMore(page.length === HISTORY_PAGE_SIZE);
      })
      .catch((err) => {
        if (!cancelled) {
          setError(messageOf(err));
        }
      });
    return () => {
      cancelled = true;
    };
  }, [active, token, roomId, clearTyping]);

  // Live connection.
  useEffect(() => {
    if (!active) {
      return;
    }
    let cancelled = false;
    let attempt = 0;
    let reconnectTimer: number | undefined;

    const dropTyping = (userId: string) => {
      const timers = typingTimersRef.current;
      const timer = timers.get(userId);
      if (timer !== undefined) {
        window.clearTimeout(timer);
        timers.delete(userId);
      }
      setTyping((prev) => {
        if (!(userId in prev)) {
          return prev;
        }
        const next = { ...prev };
        delete next[userId];
        return next;
      });
    };

    const handle = (event: ServerEvent) => {
      if (event.type === "message") {
        setMessages((prev) => mergeMessages(prev, event.message));
        // Sending ends typing immediately — no need for the TTL to expire it.
        dropTyping(event.message.user_id);
      } else if (event.type === "presence") {
        setOnline((prev) => {
          const next = { ...prev };
          if (event.event === "joined") {
            next[event.user.id] = event.user;
          } else {
            delete next[event.user.id];
          }
          return next;
        });
        if (event.event === "left") {
          dropTyping(event.user.id);
        }
      } else if (event.type === "typing") {
        const { user } = event;
        setTyping((prev) => ({ ...prev, [user.id]: user }));
        const timers = typingTimersRef.current;
        const existing = timers.get(user.id);
        if (existing !== undefined) {
          window.clearTimeout(existing);
        }
        timers.set(
          user.id,
          window.setTimeout(() => {
            timers.delete(user.id);
            setTyping((prev) => {
              if (!(user.id in prev)) {
                return prev;
              }
              const next = { ...prev };
              delete next[user.id];
              return next;
            });
          }, TYPING_TTL_MS),
        );
      } else if (event.type === "reaction") {
        setMessages((prev) =>
          applyReaction(prev, event.message_id, event.emoji, event.user_id, event.added),
        );
      } else if (event.type === "read") {
        setReads((prev) => applyRead(prev, event.message_id, event.user));
      } else {
        setError(event.message);
      }
    };

    const connect = () => {
      setStatus("connecting");
      const socket = new WebSocket(wsUrl(token, roomId));
      socketRef.current = socket;
      socket.onopen = () => {
        if (cancelled) {
          return;
        }
        attempt = 0;
        setError(null);
        setStatus("open");
      };
      socket.onmessage = (event) => {
        if (cancelled || typeof event.data !== "string") {
          return;
        }
        try {
          handle(JSON.parse(event.data) as ServerEvent);
        } catch {
          // Ignore malformed frames rather than dropping the connection.
        }
      };
      socket.onerror = () => {
        if (!cancelled) {
          setStatus("error");
        }
      };
      socket.onclose = () => {
        if (cancelled) {
          return;
        }
        setStatus("closed");
        const delay = Math.min(MAX_RECONNECT_DELAY_MS, BASE_RECONNECT_DELAY_MS * 2 ** attempt);
        attempt += 1;
        reconnectTimer = window.setTimeout(connect, delay);
      };
    };

    connect();

    return () => {
      cancelled = true;
      if (reconnectTimer !== undefined) {
        window.clearTimeout(reconnectTimer);
      }
      const socket = socketRef.current;
      socketRef.current = null;
      socket?.close();
      clearTyping();
    };
  }, [active, token, roomId, clearTyping]);

  const send = useCallback((body: string): boolean => {
    const socket = socketRef.current;
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      setError("Not connected — try again in a moment.");
      return false;
    }
    socket.send(encodeSend(body));
    return true;
  }, []);

  /** Emit a `typing` frame, at most once per `TYPING_SEND_INTERVAL_MS`. */
  const sendTyping = useCallback(() => {
    const socket = socketRef.current;
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      return;
    }
    const now = Date.now();
    if (now - typingSentAtRef.current < TYPING_SEND_INTERVAL_MS) {
      return;
    }
    typingSentAtRef.current = now;
    socket.send(encodeTyping());
  }, []);

  /** Toggle a reaction on a message; the server echoes the result back. */
  const react = useCallback((messageId: string, emoji: string) => {
    const socket = socketRef.current;
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      setError("Not connected — try again in a moment.");
      return;
    }
    socket.send(encodeReact(messageId, emoji));
  }, []);

  const loadOlder = useCallback(() => {
    const before = oldestRef.current;
    if (!active || !before || loadingOlder) {
      return;
    }
    setLoadingOlder(true);
    api
      .history(token, roomId, { limit: HISTORY_PAGE_SIZE, before })
      .then((page) => {
        setMessages((prev) => mergeMessages(prev, page));
        setHasMore(page.length === HISTORY_PAGE_SIZE);
      })
      .catch((err) => setError(messageOf(err)))
      .finally(() => setLoadingOlder(false));
  }, [active, token, roomId, loadingOlder]);

  const onlineList = useMemo(
    () => Object.values(online).sort((a, b) => a.username.localeCompare(b.username)),
    [online],
  );

  const typingList = useMemo(
    () => Object.values(typing).sort((a, b) => a.username.localeCompare(b.username)),
    [typing],
  );

  return {
    messages,
    online: onlineList,
    typing: typingList,
    reads,
    status,
    error,
    hasMore,
    loadingOlder,
    send,
    sendTyping,
    react,
    loadOlder,
  };
}
