// WebSocket URL building + frame encoding/decoding.
//
// Browsers cannot set an `Authorization` header on a WebSocket, so the JWT
// travels as a query parameter (docs/api.md → "Real-time chat").

import { API_BASE } from "./client";
import { normalizeMessage } from "./normalize";
import type { ClientSend, ServerEvent } from "./types";

/** Pure URL builder (exported for tests); `origin` is the page origin. */
export function resolveWsUrl(
  apiBase: string,
  origin: string,
  token: string,
  roomId: string,
): string {
  const url = new URL(`${apiBase}/ws`, origin);
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
  url.searchParams.set("token", token);
  url.searchParams.set("room_id", roomId);
  return url.toString();
}

/** URL for the current page's origin. */
export function wsUrl(token: string, roomId: string): string {
  return resolveWsUrl(API_BASE, window.location.origin, token, roomId);
}

/** Encode a `send` frame. */
export function encodeSend(body: string): string {
  const frame: ClientSend = { type: "send", body };
  return JSON.stringify(frame);
}

/** Encode a `typing` frame (no payload; the server knows the sender). */
export function encodeTyping(): string {
  const frame: ClientSend = { type: "typing" };
  return JSON.stringify(frame);
}

/** Encode a `react` frame that toggles `emoji` on `messageId`. */
export function encodeReact(messageId: string, emoji: string): string {
  const frame: ClientSend = { type: "react", message_id: messageId, emoji };
  return JSON.stringify(frame);
}

/**
 * Decode a server frame. Throws on malformed JSON (callers ignore the frame).
 * A `message` frame is normalized here — the WS trust boundary — so older
 * backends' omitted fields (`reactions`, #44) never reach state as
 * `undefined`.
 */
export function decodeServerEvent(raw: string): ServerEvent {
  const event = JSON.parse(raw) as ServerEvent;
  return event.type === "message" ? { ...event, message: normalizeMessage(event.message) } : event;
}
