// Pure message-list helpers.
//
// Live frames and history must agree: every message carries an `id` and a
// `created_at`, so merging is dedupe-by-id + stable chronological ordering.

import type { Message } from "../api/types";

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
