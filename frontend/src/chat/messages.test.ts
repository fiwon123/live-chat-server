import { describe, expect, it } from "vitest";

import { compareMessages, mergeMessages } from "./messages";
import type { Message } from "../api/types";

function message(overrides: Partial<Message> & { id: string; created_at: string }): Message {
  return { room_id: "room", user_id: "user", username: "alice", body: "hi", ...overrides };
}

describe("compareMessages", () => {
  it("orders older messages first", () => {
    const earlier = message({ id: "a", created_at: "2026-01-01T00:00:00.000Z" });
    const later = message({ id: "b", created_at: "2026-01-01T00:00:05.000Z" });
    expect(compareMessages(earlier, later)).toBeLessThan(0);
    expect(compareMessages(later, earlier)).toBeGreaterThan(0);
  });

  it("orders sub-second timestamps correctly (not lexicographically)", () => {
    const whole = message({ id: "a", created_at: "2026-01-01T00:00:00Z" });
    const half = message({ id: "b", created_at: "2026-01-01T00:00:00.500Z" });
    // Raw string compare would put "00.500Z" before "00Z"; chronologically it follows.
    expect(compareMessages(whole, half)).toBeLessThan(0);
  });

  it("breaks ties on id", () => {
    const a = message({ id: "a", created_at: "2026-01-01T00:00:00Z" });
    const b = message({ id: "b", created_at: "2026-01-01T00:00:00Z" });
    expect(compareMessages(a, b)).toBeLessThan(0);
  });
});

describe("mergeMessages", () => {
  it("de-duplicates by id", () => {
    const first = message({ id: "a", body: "one", created_at: "2026-01-01T00:00:00Z" });
    const again = message({ id: "a", body: "one", created_at: "2026-01-01T00:00:00Z" });
    const merged = mergeMessages([first], again);
    expect(merged).toHaveLength(1);
  });

  it("keeps the newest value for a repeated id", () => {
    const stale = message({ id: "a", body: "stale", created_at: "2026-01-01T00:00:00Z" });
    const fresh = message({ id: "a", body: "fresh", created_at: "2026-01-01T00:00:00Z" });
    expect(mergeMessages([stale], fresh)[0].body).toBe("fresh");
  });

  it("merges an array and returns chronological order", () => {
    const a = message({ id: "a", created_at: "2026-01-01T00:00:02Z" });
    const b = message({ id: "b", created_at: "2026-01-01T00:00:00Z" });
    const c = message({ id: "c", created_at: "2026-01-01T00:00:01Z" });
    expect(mergeMessages([a], [b, c]).map((m) => m.id)).toEqual(["b", "c", "a"]);
  });

  it("does not mutate its inputs", () => {
    const existing = [message({ id: "a", created_at: "2026-01-01T00:00:02Z" })];
    const incoming = message({ id: "b", created_at: "2026-01-01T00:00:00Z" });
    const snapshot = [...existing];
    mergeMessages(existing, incoming);
    expect(existing).toEqual(snapshot);
  });
});
