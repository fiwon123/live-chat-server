// Low-level HTTP plumbing shared by every endpoint.
//
// The backend's own handlers return `{ "error": "..." }`, but framework
// extractor rejections (bad content-type, malformed JSON, wrong shape) are
// plain text, so error bodies are parsed defensively.

const configured = import.meta.env.VITE_API_BASE as string | undefined;

/** API base path. Defaults to a relative `/api` (dev server proxies to the backend). */
export const API_BASE = configured && configured.length > 0 ? configured : "/api";

export class ApiError extends Error {
  readonly status: number;

  constructor(status: number, message: string) {
    super(message);
    this.name = "ApiError";
    this.status = status;
  }
}

/** Extract a human-readable message from a failed `Response`. */
export async function apiErrorFromResponse(response: Response): Promise<ApiError> {
  let body = "";
  try {
    body = await response.text();
  } catch {
    // Body already consumed or unreadable; fall back to the status line.
  }
  return new ApiError(response.status, errorText(response, body));
}

/** Pure helper (exported for tests) that picks a message out of a failed response. */
export function errorText(response: Response, body: string): string {
  const text = body.trim();
  if (text.length > 0) {
    try {
      const parsed: unknown = JSON.parse(text);
      if (parsed !== null && typeof parsed === "object" && "error" in parsed) {
        const value = (parsed as { error?: unknown }).error;
        if (typeof value === "string" && value.trim().length > 0) {
          return value.trim();
        }
      }
    } catch {
      // Plain-text extractor rejection (415/400/422/426) — use it verbatim.
    }
    return text;
  }
  return response.statusText || `request failed (${response.status})`;
}

export interface RequestOptions {
  method?: string;
  token?: string | null;
  body?: unknown;
}

/**
 * Perform a JSON request. Throws {@link ApiError} on a non-2xx response and
 * resolves `undefined` for `204 No Content`.
 */
export async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const { method = "GET", token, body } = options;
  const headers: Record<string, string> = {};
  if (body !== undefined) {
    headers["Content-Type"] = "application/json";
  }
  if (token) {
    headers.Authorization = `Bearer ${token}`;
  }

  const response = await fetch(`${API_BASE}${path}`, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
  });

  if (!response.ok) {
    throw await apiErrorFromResponse(response);
  }
  if (response.status === 204) {
    return undefined as T;
  }

  const text = await response.text();
  if (text.length === 0) {
    return undefined as T;
  }
  return JSON.parse(text) as T;
}

/** Best-effort message for an unknown thrown value. */
export function messageOf(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}
