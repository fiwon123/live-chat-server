import { afterEach, describe, expect, it, vi } from "vitest";

import { ApiError } from "./client";
import { api, historyPath } from "./endpoints";

type FetchArgs = [input: RequestInfo | URL, init?: RequestInit];

function mockFetch(response: Response) {
  const fetchMock = vi.fn<(...args: FetchArgs) => Promise<Response>>();
  fetchMock.mockResolvedValue(response);
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("historyPath", () => {
  it("omits the query string when there are no options", () => {
    expect(historyPath("room-1")).toBe("/rooms/room-1/messages");
  });

  it("includes limit and before", () => {
    expect(historyPath("room-1", { limit: 25, before: "abc" })).toBe(
      "/rooms/room-1/messages?limit=25&before=abc",
    );
  });
});

describe("api", () => {
  it("posts credentials as JSON to the login endpoint", async () => {
    const fetchMock = mockFetch(
      new Response(
        JSON.stringify({ token: "jwt", user: { id: "1", username: "alice", created_at: "x" } }),
        { status: 200, headers: { "content-type": "application/json" } },
      ),
    );

    const result = await api.login("alice", "password123");

    expect(result.token).toBe("jwt");
    const [url, init] = fetchMock.mock.calls[0];
    expect(String(url)).toBe("/api/auth/login");
    expect(init?.method).toBe("POST");
    const headers = init?.headers as Record<string, string>;
    expect(headers["Content-Type"]).toBe("application/json");
    expect(JSON.parse(String(init?.body))).toEqual({ username: "alice", password: "password123" });
  });

  it("sends the bearer token", async () => {
    const fetchMock = mockFetch(
      new Response(JSON.stringify({ id: "1", username: "alice", created_at: "x" }), {
        status: 200,
      }),
    );

    await api.me("jwt-token");

    const [, init] = fetchMock.mock.calls[0];
    const headers = init?.headers as Record<string, string>;
    expect(headers.Authorization).toBe("Bearer jwt-token");
  });

  it("surfaces the backend { error } message", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(
        async () =>
          new Response(JSON.stringify({ error: "username already taken" }), { status: 409 }),
      ),
    );

    const error = await api.register("alice", "password123").catch((caught) => caught);
    expect(error).toBeInstanceOf(ApiError);
    expect((error as ApiError).status).toBe(409);
    expect((error as ApiError).message).toBe("username already taken");
  });

  it("passes plain-text extractor rejections through", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response("Expected a JSON body", { status: 415 })),
    );

    const error = await api.login("alice", "password123").catch((caught) => caught);
    expect(error).toBeInstanceOf(ApiError);
    expect((error as ApiError).status).toBe(415);
    expect((error as ApiError).message).toBe("Expected a JSON body");
  });

  it("resolves 204 No Content as undefined", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(null, { status: 204 })),
    );
    await expect(api.joinRoom("jwt", "room-1")).resolves.toBeUndefined();
  });
});
