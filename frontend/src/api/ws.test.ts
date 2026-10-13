import { describe, expect, it } from "vitest";

import { decodeServerEvent, encodeReact, encodeSend, encodeTyping, resolveWsUrl } from "./ws";

describe("resolveWsUrl", () => {
  it("builds a ws URL with token and room_id query params", () => {
    const url = new URL(resolveWsUrl("/api", "http://localhost:5173", "tok en", "room-1"));
    expect(url.protocol).toBe("ws:");
    expect(url.pathname).toBe("/api/ws");
    expect(url.searchParams.get("token")).toBe("tok en");
    expect(url.searchParams.get("room_id")).toBe("room-1");
  });

  it("uses wss for https origins", () => {
    const url = resolveWsUrl("/api", "https://chat.example.com", "t", "r");
    expect(url.startsWith("wss://chat.example.com/api/ws?")).toBe(true);
  });

  it("honours an absolute API base", () => {
    const url = resolveWsUrl("http://localhost:8083/api", "http://localhost:5173", "t", "r");
    expect(url.startsWith("ws://localhost:8083/api/ws?")).toBe(true);
  });
});

describe("frame encoding", () => {
  it("encodes a send frame", () => {
    expect(JSON.parse(encodeSend("hi"))).toEqual({ type: "send", body: "hi" });
  });

  it("encodes a typing frame with no payload", () => {
    expect(JSON.parse(encodeTyping())).toEqual({ type: "typing" });
  });

  it("encodes a react frame with the target message and emoji", () => {
    expect(JSON.parse(encodeReact("m1", "👍"))).toEqual({
      type: "react",
      message_id: "m1",
      emoji: "👍",
    });
  });
});

describe("decodeServerEvent", () => {
  it("defaults a message frame's missing reactions to []", () => {
    // Pre-#44 backends omit `reactions` from `message` frames entirely;
    // the ChatPanel reads `message.reactions.length` and would crash.
    const raw = JSON.stringify({
      type: "message",
      message: {
        id: "m1",
        room_id: "r1",
        user_id: "u1",
        username: "alice",
        body: "hi",
        created_at: "2026-01-01T00:00:00Z",
      },
    });

    const event = decodeServerEvent(raw);
    expect(event.type).toBe("message");
    if (event.type === "message") {
      expect(event.message.reactions).toEqual([]);
    }
  });

  it("keeps existing reactions on a message frame", () => {
    const reactions = [{ emoji: "👍", count: 1, user_ids: ["u1"] }];
    const raw = JSON.stringify({
      type: "message",
      message: {
        id: "m1",
        room_id: "r1",
        user_id: "u1",
        username: "alice",
        body: "hi",
        created_at: "2026-01-01T00:00:00Z",
        reactions,
      },
    });

    const event = decodeServerEvent(raw);
    if (event.type === "message") {
      expect(event.message.reactions).toEqual(reactions);
    }
  });

  it("passes non-message frames through untouched", () => {
    const raw = JSON.stringify({ type: "typing", user: { id: "u1", username: "alice" } });
    expect(decodeServerEvent(raw)).toEqual({
      type: "typing",
      user: { id: "u1", username: "alice" },
    });
  });

  it("passes a room_deleted frame through with its room id", () => {
    const raw = JSON.stringify({ type: "room_deleted", room_id: "r1" });
    expect(decodeServerEvent(raw)).toEqual({ type: "room_deleted", room_id: "r1" });
  });

  it("throws on malformed JSON so callers can ignore the frame", () => {
    expect(() => decodeServerEvent("not json")).toThrow();
  });
});
