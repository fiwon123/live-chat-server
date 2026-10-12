// Shared API types, mirroring the response shapes in docs/api.md.

export interface User {
  id: string;
  username: string;
  created_at: string;
}

export interface AuthResponse {
  token: string;
  user: User;
}

export interface Room {
  id: string;
  name: string;
  created_by: string;
  created_at: string;
  member: boolean;
  /** Messages newer than the viewer's read cursor (server-counted). */
  unread_count: number;
  /** Newest message in the room, for list previews; `null` when empty. */
  last_message: LastMessage | null;
}

/** Compact latest-message preview attached to a room. */
export interface LastMessage {
  id: string;
  user_id: string;
  username: string;
  body: string;
  created_at: string;
}

/** Outcome of marking a room read (`POST /rooms/{id}/read`). */
export interface ReadReceipt {
  room_id: string;
  last_read_message_id: string | null;
  unread_count: number;
}

export interface Message {
  id: string;
  room_id: string;
  user_id: string;
  username: string;
  body: string;
  created_at: string;
  /** Grouped reactions; empty when nobody has reacted. */
  reactions: ReactionGroup[];
}

/** One emoji's reactions on a message. */
export interface ReactionGroup {
  emoji: string;
  count: number;
  user_ids: string[];
}

export interface PresenceUser {
  id: string;
  username: string;
}

/** Frames the server sends (see docs/api.md → "Server → client frames"). */
export type ServerEvent =
  | { type: "message"; message: Message }
  | { type: "presence"; event: "joined" | "left"; user: PresenceUser }
  | { type: "typing"; user: PresenceUser }
  | {
      type: "reaction";
      message_id: string;
      emoji: string;
      user_id: string;
      username: string;
      added: boolean;
    }
  | { type: "read"; message_id: string; user: PresenceUser }
  | { type: "error"; message: string };

/** Frames the client sends (see docs/api.md → "Client → server frames"). */
export type ClientSend =
  | { type: "send"; body: string }
  | { type: "typing" }
  | { type: "react"; message_id: string; emoji: string };
