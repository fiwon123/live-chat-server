# API Reference

Base URL: `/api` (the lone health probe lives at `/health`).

All request bodies and responses are JSON (`application/json`). The handlers'
own error responses use a single shape:

```json
{ "error": "<client-safe message>" }
```

Internal error details are logged server-side and never returned.

> **Not every failure looks like that.** Request parsing is done by framework
> extractors, whose rejections return **plain text** (not `{"error": ...}`):
> a missing/incorrect `Content-Type: application/json` → `415`; a malformed
> JSON body or a malformed `{id}`/`before` UUID → `400`; valid JSON of the wrong
> shape → `422`. Clients should parse error bodies defensively. `Content-Type:
> application/json` is **required** on `register`, `login`, and `POST /api/rooms`.

## Authentication

All endpoints except `register`, `login`, and `health` require a bearer token:

```
Authorization: Bearer <jwt>
```

Tokens are HS256 JWTs issued at register and login, signed with
`JWT_SECRET` and expiring **24 hours** after issue (`exp` is validated on every
use). Set `JWT_SECRET` to a strong random value before deploying; the server
logs a warning when it is unset (it then uses the committed dev-only default).
The scheme is matched case-insensitively (`Bearer`/`bearer`).

### `POST /api/auth/register`

Create a user and return a token.

Request:

```json
{ "username": "alice", "password": "correct horse battery staple" }
```

- `username`: 3–32 chars of `[a-z0-9_]` (lowercase ASCII only).
- `password`: 8–128 chars (capped because argon2 cost grows with input).

Responses:

| Status | Body |
|--------|------|
| 201 | `{ "token": "<jwt>", "user": { "id": "<uuid>", "username": "alice", "created_at": "<rfc3339>" } }` |
| 400 | validation error (`{"error": "..."}`) |
| 409 | username already taken |

### `POST /api/auth/login`

Same request/response shape as register; `200` on success, `401` on bad
credentials.

### `GET /api/me`

Returns the authenticated user; `200` with the `user` object shape above,
`401` without a valid token.

## Rooms

### `POST /api/rooms`

Create a room. The creator automatically becomes a member (atomic in storage).

Request: `{ "name": "general" }` — name trimmed, non-empty, ≤ 64 chars, no
control characters.

Response `201`:

```json
{
  "id": "<uuid>",
  "name": "general",
  "created_by": "<uuid>",
  "created_at": "<rfc3339>",
  "member": true,
  "unread_count": 0,
  "last_message": null
}
```

`400` on invalid names.

### `GET /api/rooms`

List every room with the caller's membership flag (`member: true|false`),
per-room `unread_count`, and a `last_message` preview.

`200` with an array of room objects:

```json
{
  "id": "<uuid>",
  "name": "general",
  "created_by": "<uuid>",
  "created_at": "<rfc3339>",
  "member": true,
  "unread_count": 3,
  "last_message": {
    "id": "<uuid>",
    "user_id": "<uuid>",
    "username": "alice",
    "body": "hello",
    "created_at": "<rfc3339>"
  }
}
```

- `unread_count`: the caller's unread total — messages **from other users**
  newer than the caller's read cursor. `0` for a fresh room and for non-members;
  a member who never opened the room sees everything as unread.
- `last_message`: the newest message in the room, or `null` when the room is
  empty. Present **only for member rooms** — non-members get `null`, so the
  list previews rooms only after joining (message bodies and unread counts are
  member-gated, matching history access).

### `GET /api/rooms/{id}`

One room; `200` with the room object (same shape as above, including the
member-gated `unread_count` / `last_message`), `404` for an unknown id. No
membership required to *see* a room, but non-members see no preview.

### `POST /api/rooms/{id}/join`

Join a room. **Idempotent**: joining twice is still `204`. `204` on success,
`404` for an unknown room. Joining is open — any authenticated user may join.

### `POST /api/rooms/{id}/leave`

Leave a room. **Idempotent**: `204` even if you are not a member; `404` for an
unknown room.

### `POST /api/rooms/{id}/read`

Mark messages read in a room you are a **member** of. Advances the caller's read
cursor — never backwards — and broadcasts an ephemeral `read` receipt to the
room (see Server → client frames).

Request (either form):

```json
{ "message_id": "<uuid>" }
```

```json
{}
```

- With `message_id`: mark everything up to (and including) that message read.
  The message must exist **in this room**, else `400`.
- `{}` or `{ "message_id": null }`: mark everything read (the cursor becomes the
  newest message). A room with no messages is a no-op.

Response `200`:

```json
{
  "room_id": "<uuid>",
  "last_read_message_id": "<uuid>",
  "unread_count": 0
}
```

`last_read_message_id` is `null` when the cursor did not move — an empty room,
or a late/duplicate/out-of-order mark that cannot move the cursor backwards.
`unread_count` is the caller's remaining unread total after the cursor moved.

| Status | Meaning |
|--------|---------|
| 200 | cursor advanced; receipt broadcast |
| 400 | `message_id` does not belong to this room |
| 401 | missing/invalid token |
| 403 | caller is not a member |
| 404 | unknown room |

## Message history

### `GET /api/rooms/{id}/messages?limit=&before=`

Message history for a **member**. Newest first.

Query parameters:

- `limit` (optional): page size; clamped server-side to 1–100, default 50.
- `before` (optional): UUID message-id cursor; returns messages strictly older
  than that message. A well-formed cursor that matches no message (or belongs
  to another room) yields an empty page.

Response `200`:

```json
[
  {
    "id": "<uuid>",
    "room_id": "<uuid>",
    "user_id": "<uuid>",
    "username": "alice",
    "body": "hello",
    "created_at": "<rfc3339>",
    "reactions": [
      { "emoji": "👍", "count": 2, "user_ids": ["<uuid>", "<uuid>"] }
    ]
  },
  ...
]
```

`reactions` groups one entry per emoji on that message; `count` always equals
`user_ids.length`, and `user_ids` lists every reactor (so the client can
highlight the caller's own reactions). Messages with no reactions carry `[]`.

| Status | Meaning |
|--------|---------|
| 200 | page of messages (possibly empty) |
| 403 | caller is not a member of the room |
| 404 | unknown room |

## Real-time chat (WebSocket)

### `GET /api/ws?token=<jwt>&room_id=<uuid>`

Upgrades the connection to a WebSocket. Browsers cannot set an `Authorization`
header on WS connections, so the JWT travels as a query parameter — keep it out
of logs and URLs where possible.

Handshake checks, in order, **before** the upgrade:

1. `token` must be a valid JWT → else `401`.
2. The user must still exist → else `401`.
3. The room must exist → else `404`.
4. The user must be a member of the room → else `403`.

A successful upgrade answers `101 Switching Protocols`.

The request must be a genuine WebSocket upgrade (`Connection: Upgrade`,
`Upgrade: websocket`, `Sec-WebSocket-Version`, `Sec-WebSocket-Key`); a plain
`GET` to this path is rejected by request extraction (`400`/`426`) *before* the
token check runs, and a missing or malformed `token`/`room_id` parameter yields
a plain-text `400`.

### Client → server frames

Three message types, as text frames:

```json
{ "type": "send", "body": "hello everyone" }
```

```json
{ "type": "typing" }
```

```json
{ "type": "react", "message_id": "<uuid>", "emoji": "👍" }
```

Validation for `send` (per message, on every send):

- `body` is trimmed, must be non-empty.
- At most **4000 characters**.
- No control characters (matches room-name rules; NUL must not reach Postgres).

`typing` carries no payload — the server knows the sender. It is throttled to at
most one relayed event per session every **1.5 s**; anything faster is dropped
silently, and the connection stays open.

`react` **toggles** `emoji` on `message_id` for the calling user: absent → added,
present → removed. `emoji` is trimmed, must be non-empty, at most **16 chars**,
and control-character free. The target message must exist **in the caller's
current room**; otherwise an `error` frame is returned and nothing changes.

### Server → client frames

Text frames, JSON, tagged by `type` (`snake_case`):

```json
{ "type": "message", "message": { "id": "<uuid>", "room_id": "<uuid>", "user_id": "<uuid>", "username": "alice", "body": "hello everyone", "created_at": "<rfc3339>", "reactions": [] } }
```

```json
{ "type": "presence", "event": "joined", "user": { "id": "<uuid>", "username": "alice" } }
```

```json
{ "type": "presence", "event": "left", "user": { "id": "<uuid>", "username": "alice" } }
```

```json
{ "type": "error", "message": "message must not be empty" }
```

```json
{ "type": "typing", "user": { "id": "<uuid>", "username": "alice" } }
```

```json
{ "type": "reaction", "message_id": "<uuid>", "emoji": "👍", "user_id": "<uuid>", "username": "alice", "added": true }
```

```json
{ "type": "read", "message_id": "<uuid>", "user": { "id": "<uuid>", "username": "alice" } }
```

Semantics:

- **Save before broadcast**: a `send` frame is inserted into Postgres first;
  the broadcast carries the row's `id` and `created_at` (RFC 3339). Every
  client — including the sender — receives the identical `message` event, so
  live view and reconnect history (`/api/rooms/{id}/messages`) always agree.
- **Reactions**: toggling `react` persists the reaction row (ON CONFLICT DO NOTHING
  for inserts, DELETE on removal); a `reaction` event is broadcast room-wide
  (`message_id`, `emoji`, `user_id`, `username`, `added`). The event identifies
  the actor so clients can update their local lists. Rendering uses
  `user_ids.length` to compute `count`.
- **Error frames are non-fatal**: a failed `send` (blank, too long, malformed
  JSON, unparseable shape) or an invalid `react` produces an `error` event and
  the connection stays open. Invalid messages/reactions are never persisted.
- **Presence is reference-counted**: a user with several open tabs sees
  `joined` on their first connection and `left` after their last one closes —
  no spurious chatter when a tab closes.
- **Membership is enforced on both directions**: leaving the room over HTTP
  (`POST /api/rooms/{id}/leave`) means the next room message closes the
  session instead of being delivered. Connections stop at ~64 KiB per message
  (server-enforced frame cap).
- **Typing is ephemeral**: `typing` events are relayed room-wide, never
  persisted, and excluded from history. The server throttles them per session
  (1.5 s) and does **not** echo a sender's own event back to them; clients
  should also ignore any `typing` frame whose `user.id` is their own. There is
  no explicit "stopped typing" frame — send a message or let the client
  timeout do it.
- **Read receipts are ephemeral**: `POST /api/rooms/{id}/read` persists the
  caller's cursor (the source of truth for unread counts) and broadcasts a
  `read` frame (`message_id` = the new cursor, only when the cursor actually
  advanced) so room members can show who has seen a message. The receipt is
  best-effort: a member with no open connections simply misses it, and clients
  should ignore a `read` frame whose `user.id` is their own. There is no
  client → server read frame; marking read is always the HTTP endpoint.
- Events are broadcast room-wide only; room A clients never see room B's
  traffic. There is no history replay on connect — fetch
  `GET /api/rooms/{id}/messages` after upgrading if needed.

Keepalive: the server responds to protocol ping frames; there are no
periodic application-level heartbeats.

## Health

### `GET /health`

`200` with `{ "status": "ok", "service": "live-chat-server", "version": "<semver>" }`.

## Reference

- `RoomResponse`: `{ id, name, created_by, created_at, member, unread_count, last_message }`
- `LastMessageResponse`: `{ id, user_id, username, body, created_at }`
- `ReadResponse`: `{ room_id, last_read_message_id, unread_count }`
- `MessageResponse`: `{ id, room_id, user_id, username, body, created_at }`
- Path ids and the `before` cursor must be well-formed UUIDs.
- Handler errors: `{ "error": "<message>" }` with status 400/401/403/404/409/500.
- Extractor rejections (bad content-type/JSON/query/UUID): plain text with
  400/415/422 (and 426 for a non-upgrade request to `/api/ws`).