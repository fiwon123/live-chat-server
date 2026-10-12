// Typed wrappers around every endpoint in docs/api.md.

import { request } from "./client";
import type { AuthResponse, LastMessage, Message, ReadReceipt, Room, User } from "./types";

// Payload shapes as version-skewed backends may actually emit them. Fields
// introduced after v0.1 (e.g. `last_message` from #45, `reactions` from #44)
// are absent from older backends' JSON — an absent key yields `undefined` in
// JS, which the declared `Room`/`Message` types do not express. Normalizing at
// the fetch boundary below closes that gap so components can trust the
// declared types.
type RoomPayload = Omit<Room, "last_message" | "unread_count"> & {
  last_message?: LastMessage | null;
  unread_count?: number;
};

type MessagePayload = Omit<Message, "reactions"> & {
  reactions?: Message["reactions"];
};

/** Normalize a room payload; missing/invalid optional fields degrade to safe
 * defaults (`last_message: null`, `unread_count: 0`). */
export function normalizeRoom(room: RoomPayload): Room {
  const unread = room.unread_count;
  return {
    ...room,
    unread_count:
      typeof unread === "number" && Number.isFinite(unread) ? Math.max(0, Math.floor(unread)) : 0,
    last_message: room.last_message ?? null,
  };
}

/** Normalize a message payload; a missing `reactions` key becomes `[]`. */
export function normalizeMessage(message: MessagePayload): Message {
  return { ...message, reactions: message.reactions ?? [] };
}

/** Build the history path, including optional `limit` / `before` query params. */
export function historyPath(
  roomId: string,
  options: { limit?: number; before?: string | null } = {},
): string {
  const params = new URLSearchParams();
  if (options.limit !== undefined) {
    params.set("limit", String(options.limit));
  }
  if (options.before) {
    params.set("before", options.before);
  }
  const query = params.toString();
  return `/rooms/${roomId}/messages${query.length > 0 ? `?${query}` : ""}`;
}

export const api = {
  register(username: string, password: string) {
    return request<AuthResponse>("/auth/register", {
      method: "POST",
      body: { username, password },
    });
  },

  login(username: string, password: string) {
    return request<AuthResponse>("/auth/login", {
      method: "POST",
      body: { username, password },
    });
  },

  me(token: string) {
    return request<User>("/me", { token });
  },

  listRooms(token: string) {
    return request<RoomPayload[]>("/rooms", { token }).then((rooms) => rooms.map(normalizeRoom));
  },

  createRoom(token: string, name: string) {
    return request<RoomPayload>("/rooms", { method: "POST", token, body: { name } }).then(
      normalizeRoom,
    );
  },

  joinRoom(token: string, roomId: string) {
    return request<void>(`/rooms/${roomId}/join`, { method: "POST", token });
  },

  leaveRoom(token: string, roomId: string) {
    return request<void>(`/rooms/${roomId}/leave`, { method: "POST", token });
  },

  history(token: string, roomId: string, options: { limit?: number; before?: string | null } = {}) {
    return request<MessagePayload[]>(historyPath(roomId, options), { token }).then((page) =>
      page.map(normalizeMessage),
    );
  },

  /**
   * Mark a room read. `messageId` advances the cursor to a specific message;
   * when omitted, everything is marked read. The server broadcasts an ephemeral
   * `read` receipt to the room and returns the effective cursor + unread total.
   */
  read(token: string, roomId: string, messageId?: string | null) {
    return request<ReadReceipt>(`/rooms/${roomId}/read`, {
      method: "POST",
      token,
      body: messageId ? { message_id: messageId } : {},
    });
  },
};
