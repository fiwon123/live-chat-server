// Trust-boundary normalization for payloads the server sends.
//
// Fields introduced after v0.1 (e.g. `last_message` from #45, `reactions`
// from #44) are absent from older backends' JSON — an absent key yields
// `undefined` in JS, which the declared `Room`/`Message` types do not express.
// Normalizing where payloads enter the app (HTTP responses, WS frames) closes
// that gap so components can trust the declared types.

import type { LastMessage, Message, Room } from "./types";

/** Room shape as version-skewed backends may actually emit it. */
export type RoomPayload = Omit<Room, "last_message" | "unread_count"> & {
  last_message?: LastMessage | null;
  unread_count?: number;
};

/** Message shape as version-skewed backends may actually emit it. */
export type MessagePayload = Omit<Message, "reactions"> & {
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
