import { describe, expect, it } from "vitest";

import {
  compareMessages,
  formatDayLabel,
  formatMessageTime,
  isSameDay,
  mergeMessages,
  shouldGroup,
  typingLabel,
} from "./messages";
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

describe("isSameDay", () => {
  it("treats two messages on one day as the same day", () => {
    const a = message({ id: "a", created_at: "2026-03-10T23:59:00Z" });
    const b = message({ id: "b", created_at: "2026-03-10T00:01:00Z" });
    expect(isSameDay(a, b)).toBe(true);
  });

  it("separates messages on different days", () => {
    const a = message({ id: "a", created_at: "2026-03-10T23:59:00Z" });
    const b = message({ id: "b", created_at: "2026-03-11T00:00:00Z" });
    expect(isSameDay(a, b)).toBe(false);
  });

  it("returns false on invalid timestamps", () => {
    const a = message({ id: "a", created_at: "not-a-date" });
    const b = message({ id: "b", created_at: "2026-03-10T00:00:00Z" });
    expect(isSameDay(a, b)).toBe(false);
  });
});

describe("shouldGroup", () => {
  const at = (iso: string) => message({ id: iso, created_at: iso });

  it("groups same-author messages within the gap", () => {
    expect(shouldGroup(at("2026-03-10T10:00:00Z"), at("2026-03-10T10:03:00Z"))).toBe(true);
  });

  it("does not group distant same-author messages", () => {
    expect(shouldGroup(at("2026-03-10T10:00:00Z"), at("2026-03-10T10:20:00Z"))).toBe(false);
  });

  it("never groups different authors, even adjacent", () => {
    const a = message({ id: "a", user_id: "u1", created_at: "2026-03-10T10:00:00Z" });
    const b = message({ id: "b", user_id: "u2", created_at: "2026-03-10T10:01:00Z" });
    expect(shouldGroup(a, b)).toBe(false);
  });

  it("respects an explicit gap", () => {
    const a = at("2026-03-10T10:00:00Z");
    const b = at("2026-03-10T10:02:00Z");
    expect(shouldGroup(a, b, 1)).toBe(false);
    expect(shouldGroup(a, b, 3)).toBe(true);
  });
});

describe("formatMessageTime", () => {
  it("formats to HH:MM and ignores sub-second precision differences", () => {
    expect(formatMessageTime("2026-03-10T10:05:30Z")).toMatch(/^\d{2}:\d{2}$/);
    expect(formatMessageTime("2026-03-10T10:05:30.123Z")).toMatch(/^\d{2}:\d{2}$/);
  });

  it("returns an empty string for invalid input", () => {
    expect(formatMessageTime("nope")).toBe("");
  });
});

describe("formatDayLabel", () => {
  const noon = (dayOffset: number) => {
    const date = new Date();
    date.setHours(12, 0, 0, 0);
    date.setDate(date.getDate() + dayOffset);
    return date.toISOString();
  };

  it("labels today and yesterday", () => {
    expect(formatDayLabel(noon(0))).toBe("Today");
    expect(formatDayLabel(noon(-1))).toBe("Yesterday");
  });

  it("labels older days by date", () => {
    expect(formatDayLabel(noon(-6))).toMatch(/^[A-Z][a-z]{2}, [A-Z][a-z]{2} \d{1,2}$/);
  });
});

describe("typingLabel", () => {
  const user = (username: string) => ({ id: username, username });

  it("returns null when nobody is typing", () => {
    expect(typingLabel([])).toBeNull();
  });

  it("names one typer", () => {
    expect(typingLabel([user("alice")])).toBe("alice is typing…");
  });

  it("names two typers", () => {
    expect(typingLabel([user("alice"), user("bob")])).toBe("alice and bob are typing…");
  });

  it("collapses three or more", () => {
    expect(typingLabel([user("a"), user("b"), user("c")])).toBe("Several people are typing…");
  });
});
