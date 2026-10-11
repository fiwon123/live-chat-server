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
}

export interface Message {
  id: string;
  room_id: string;
  user_id: string;
  username: string;
  body: string;
  created_at: string;
}

export interface PresenceUser {
  id: string;
  username: string;
}

/** Frames the server sends (see docs/api.md → "Server → client frames"). */
export type ServerEvent =
  | { type: "message"; message: Message }
  | { type: "presence"; event: "joined" | "left"; user: PresenceUser }
  | { type: "error"; message: string };

/** Frames the client sends (see docs/api.md → "Client → server frames"). */
export interface ClientSend {
  type: "send";
  body: string;
}
