// Live chat room: history fetch + WebSocket subscription for one room.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { messageOf } from "../api/client";
import { api } from "../api/endpoints";
import type { Message, PresenceUser, ServerEvent } from "../api/types";
import { encodeSend, wsUrl } from "../api/ws";
import { mergeMessages } from "../chat/messages";

export const HISTORY_PAGE_SIZE = 50;
const BASE_RECONNECT_DELAY_MS = 500;
const MAX_RECONNECT_DELAY_MS = 8000;

export type ConnectionStatus = "idle" | "connecting" | "open" | "closed" | "error";

export interface ChatRoom {
  messages: Message[];
  online: PresenceUser[];
  status: ConnectionStatus;
  error: string | null;
  hasMore: boolean;
  loadingOlder: boolean;
  send: (body: string) => boolean;
  loadOlder: () => void;
}

/**
 * Connect to a room's WebSocket and load its history. Reconnects with capped
 * exponential backoff; a room change tears the connection down.
 */
export function useChatRoom(token: string, roomId: string): ChatRoom {
  const [messages, setMessages] = useState<Message[]>([]);
  const [online, setOnline] = useState<Record<string, PresenceUser>>({});
  const [status, setStatus] = useState<ConnectionStatus>("idle");
  const [error, setError] = useState<string | null>(null);
  const [hasMore, setHasMore] = useState(false);
  const [loadingOlder, setLoadingOlder] = useState(false);

  const socketRef = useRef<WebSocket | null>(null);
  const oldestRef = useRef<string | null>(null);

  const active = token.length > 0 && roomId.length > 0;

  useEffect(() => {
    oldestRef.current = messages.length > 0 ? messages[0].id : null;
  }, [messages]);

  // History for the (new) room.
  useEffect(() => {
    setMessages([]);
    setOnline({});
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
  }, [active, token, roomId]);

  // Live connection.
  useEffect(() => {
    if (!active) {
      return;
    }
    let cancelled = false;
    let attempt = 0;
    let reconnectTimer: number | undefined;

    const handle = (event: ServerEvent) => {
      if (event.type === "message") {
        setMessages((prev) => mergeMessages(prev, event.message));
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
        if (typeof event.data !== "string") {
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
    };
  }, [active, token, roomId]);

  const send = useCallback((body: string): boolean => {
    const socket = socketRef.current;
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      setError("Not connected — try again in a moment.");
      return false;
    }
    socket.send(encodeSend(body));
    return true;
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

  return { messages, online: onlineList, status, error, hasMore, loadingOlder, send, loadOlder };
}
