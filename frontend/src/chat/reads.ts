// Read-receipt tracking.
//
// The server persists each member's read cursor (newest message they have
// seen) and broadcasts an ephemeral `read` frame when it advances. Clients
// keep the latest cursor per reader and derive "Seen" markers on their own
// messages from those cursors.

import type { Message, PresenceUser } from "../api/types";

/** A reader's cursor: the newest message they have confirmed reading. */
export interface ReadCursor {
  message_id: string;
  username: string;
}

/** Readers keyed by user id (the server sends the latest cursor, so storing
 *  one per reader is enough). */
export type ReadMap = Record<string, ReadCursor>;

/** Record that `user` has read up to `messageId`, replacing any older cursor. */
export function applyRead(map: ReadMap, messageId: string, user: PresenceUser): ReadMap {
  return { ...map, [user.id]: { message_id: messageId, username: user.username } };
}

/**
 * The users whose read cursor has reached `messageId` (excluding `selfId`).
 *
 * Cursors are compared by position in the loaded `messages` list; a cursor
 * whose message is not loaded cannot be compared and is conservatively treated
 * as "not seen". This is fine in practice because receipts target the newest
 * message, which is always loaded.
 */
export function seenBy(
  messages: Message[],
  reads: ReadMap,
  messageId: string,
  selfId: string,
): PresenceUser[] {
  const index = new Map<string, number>();
  messages.forEach((message, position) => index.set(message.id, position));
  const target = index.get(messageId);
  if (target === undefined) {
    return [];
  }
  return Object.entries(reads)
    .filter(([readerId]) => readerId !== selfId)
    .filter(([, cursor]) => {
      const cursorIndex = index.get(cursor.message_id);
      return cursorIndex !== undefined && cursorIndex >= target;
    })
    .map(([id, cursor]) => ({ id, username: cursor.username }))
    .sort((a, b) => a.username.localeCompare(b.username));
}
