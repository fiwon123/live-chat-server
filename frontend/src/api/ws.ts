// WebSocket URL building + client frame encoding.
//
// Browsers cannot set an `Authorization` header on a WebSocket, so the JWT
// travels as a query parameter (docs/api.md → "Real-time chat").

import { API_BASE } from "./client";
import type { ClientSend } from "./types";

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
