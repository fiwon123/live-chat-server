import { describe, expect, it } from "vitest";

import { encodeReact, encodeSend, encodeTyping, resolveWsUrl } from "./ws";

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
