// Pure reaction-list helpers.
//
// The server broadcasts the authoritative `added` flag; the UI applies it to
// the matching message so every client converges on the same grouped state.

import type { Message, ReactionGroup } from "../api/types";

/**
 * The quick-reaction bar offered on hover. It is only shown on other people's
 * messages — you cannot react to your own — so users type custom emoji in chat.
 */
export const QUICK_REACTIONS = ["👍", "❤️", "😂", "🎉", "👀"] as const;

/**
 * Whether `userId` has reacted with `emoji` on `group`.
 */
export function hasReacted(group: ReactionGroup, userId: string): boolean {
  return group.user_ids.includes(userId);
}

/** Coerce a group's `count` to its `user_ids` length (one reaction per user). */
function withCount(group: ReactionGroup): ReactionGroup {
  return { ...group, count: group.user_ids.length };
}

/**
 * Apply a single reaction toggle to the message with `messageId`. Returns the
 * same array reference when nothing matched, so React can skip re-rendering.
 */
export function applyReaction(
  messages: Message[],
  messageId: string,
  emoji: string,
  userId: string,
  added: boolean,
): Message[] {
  let changed = false;
  const next = messages.map((message) => {
    if (message.id !== messageId) {
      return message;
    }
    const groups: ReactionGroup[] = message.reactions.map((group) => ({
      ...group,
      user_ids: [...group.user_ids],
    }));
    const group = groups.find((candidate) => candidate.emoji === emoji);
    if (added) {
      if (group === undefined) {
        groups.push({ emoji, count: 1, user_ids: [userId] });
      } else if (!group.user_ids.includes(userId)) {
        group.user_ids.push(userId);
      } else {
        return message; // Already reacted; nothing to do.
      }
    } else {
      if (group === undefined) {
        return message; // No such reaction to remove.
      }
      group.user_ids = group.user_ids.filter((id) => id !== userId);
    }
    changed = true;
    return {
      ...message,
      reactions: groups.filter((candidate) => candidate.user_ids.length > 0).map(withCount),
    };
  });
  return changed ? next : messages;
}
