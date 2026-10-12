// Typed wrappers around every endpoint in docs/api.md.

import { request } from "./client";
import type { AuthResponse, Message, ReadReceipt, Room, User } from "./types";

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
    return request<Room[]>("/rooms", { token });
  },

  createRoom(token: string, name: string) {
    return request<Room>("/rooms", { method: "POST", token, body: { name } });
  },

  joinRoom(token: string, roomId: string) {
    return request<void>(`/rooms/${roomId}/join`, { method: "POST", token });
  },

  leaveRoom(token: string, roomId: string) {
    return request<void>(`/rooms/${roomId}/leave`, { method: "POST", token });
  },

  history(token: string, roomId: string, options: { limit?: number; before?: string | null } = {}) {
    return request<Message[]>(historyPath(roomId, options), { token });
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
