// Pure message-list helpers.
//
// Live frames and history must agree: every message carries an `id` and a
// `created_at`, so merging is dedupe-by-id + stable chronological ordering.

import type { Message, PresenceUser } from "../api/types";

/**
 * Chronological order, oldest first. `created_at` is RFC 3339 with variable
 * fractional-second precision, so it is parsed rather than string-compared;
 * ties break on `id` for a total order.
 */
export function compareMessages(a: Message, b: Message): number {
  const at = Date.parse(a.created_at);
  const bt = Date.parse(b.created_at);
  if (Number.isNaN(at) || Number.isNaN(bt)) {
    return a.created_at.localeCompare(b.created_at) || a.id.localeCompare(b.id);
  }
  return at - bt || a.id.localeCompare(b.id);
}

/** Merge one or more messages into an existing list, de-duplicated by id. */
export function mergeMessages(existing: Message[], incoming: Message | Message[]): Message[] {
  const byId = new Map<string, Message>();
  for (const message of existing) {
    byId.set(message.id, message);
  }
  const additions = Array.isArray(incoming) ? incoming : [incoming];
  for (const message of additions) {
    byId.set(message.id, message);
  }
  return [...byId.values()].sort(compareMessages);
}

/** Coarse "same calendar day" test for date separators. */
export function isSameDay(a: Message, b: Message): boolean {
  const ad = new Date(a.created_at);
  const bd = new Date(b.created_at);
  if (Number.isNaN(ad.getTime()) || Number.isNaN(bd.getTime())) {
    return false;
  }
  return (
    ad.getFullYear() === bd.getFullYear() &&
    ad.getMonth() === bd.getMonth() &&
    ad.getDate() === bd.getDate()
  );
}

/**
 * Whether two adjacent messages belong to one visual cluster: same author,
 * sent within `gapMinutes` of each other (messages arrive sorted).
 */
export function shouldGroup(prev: Message, next: Message, gapMinutes = 5): boolean {
  if (prev.user_id !== next.user_id) {
    return false;
  }
  const gap = Date.parse(next.created_at) - Date.parse(prev.created_at);
  return !Number.isNaN(gap) && gap >= 0 && gap <= gapMinutes * 60_000;
}

/** "14:32" style local time, stable regardless of input precision. */
export function formatMessageTime(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    return "";
  }
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", hour12: false });
}

/** "Today" / "Yesterday" / "Wed, Jun 4" style separator label. */
export function formatDayLabel(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    return "";
  }
  const startOfDay = (value: Date) =>
    new Date(value.getFullYear(), value.getMonth(), value.getDate()).getTime();
  const day = startOfDay(date);
  const today = startOfDay(new Date());
  const yesterday = today - 24 * 60 * 60 * 1000;

  if (day === today) {
    return "Today";
  }
  if (day === yesterday) {
    return "Yesterday";
  }
  return date.toLocaleDateString([], { weekday: "short", month: "short", day: "numeric" });
}

/**
 * One-line description of who is typing, or `null` when nobody is.
 * 1 → "alice is typing…", 2 → "alice and bob are typing…", 3+ → collapsed.
 */
export function typingLabel(users: PresenceUser[]): string | null {
  if (users.length === 0) {
    return null;
  }
  if (users.length === 1) {
    return `${users[0].username} is typing…`;
  }
  if (users.length === 2) {
    return `${users[0].username} and ${users[1].username} are typing…`;
  }
  return "Several people are typing…";
}
