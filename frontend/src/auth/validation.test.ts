import { describe, expect, it } from "vitest";

import { credentialRules, credentialsValid, isValidPassword, isValidUsername } from "./validation";

describe("isValidUsername", () => {
  it("accepts 3–32 chars of lowercase, digits and underscore", () => {
    expect(isValidUsername("ann")).toBe(true);
    expect(isValidUsername("user_42")).toBe(true);
    expect(isValidUsername("a".repeat(32))).toBe(true);
  });

  it("rejects too short, too long, uppercase and symbols", () => {
    expect(isValidUsername("ab")).toBe(false);
    expect(isValidUsername("a".repeat(33))).toBe(false);
    expect(isValidUsername("Anna")).toBe(false);
    expect(isValidUsername("user-42")).toBe(false);
    expect(isValidUsername("user 42")).toBe(false);
    expect(isValidUsername("")).toBe(false);
  });
});

describe("isValidPassword", () => {
  it("requires at least 8 and at most 128 characters", () => {
    expect(isValidPassword("1234567")).toBe(false);
    expect(isValidPassword("12345678")).toBe(true);
    expect(isValidPassword("x".repeat(128))).toBe(true);
    expect(isValidPassword("x".repeat(129))).toBe(false);
  });
});

describe("credentialRules", () => {
  it("reports each rule's met state", () => {
    const rules = credentialRules("ann", "password123");
    expect(rules.map((rule) => rule.met)).toEqual([true, true]);

    const partial = credentialRules("an", "short");
    expect(partial.map((rule) => rule.met)).toEqual([false, false]);
  });
});

describe("credentialsValid", () => {
  it("requires both fields to be valid", () => {
    expect(credentialsValid("ann", "password123")).toBe(true);
    expect(credentialsValid("an", "password123")).toBe(false);
    expect(credentialsValid("ann", "short")).toBe(false);
  });
});
