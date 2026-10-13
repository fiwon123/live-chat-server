import { describe, expect, it } from "vitest";

import { routeFromHash } from "./useHashRoute";

describe("routeFromHash", () => {
  it("maps #/about to the About route", () => {
    expect(routeFromHash("#/about")).toBe("about");
    expect(routeFromHash("#/about/")).toBe("about");
    expect(routeFromHash("#/About")).toBe("about");
  });

  it("maps anything else to the chat route", () => {
    expect(routeFromHash("")).toBe("chat");
    expect(routeFromHash("#")).toBe("chat");
    expect(routeFromHash("#/")).toBe("chat");
    expect(routeFromHash("#/chat")).toBe("chat");
    expect(routeFromHash("#room-42")).toBe("chat");
  });
});
