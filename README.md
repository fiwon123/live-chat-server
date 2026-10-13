<div align="center">

<img src="frontend/public/favicon.svg" alt="live-chat-server logo — teal speech bubble" width="72">

# live-chat-server

**Real-time chat with a Rust backend and a React + TypeScript client.**
WebSockets, JWT auth, room-based access control, and message persistence in PostgreSQL.

![CI](https://github.com/fiwon123/live-chat-server/actions/workflows/ci.yml/badge.svg?branch=main)
![Rust](https://img.shields.io/badge/Rust-1.99-orange?logo=rust&logoColor=white)
![Axum](https://img.shields.io/badge/axum-0.8-red)
![SQLx](https://img.shields.io/badge/sqlx-0.9-8f4f2f)
![PostgreSQL](https://img.shields.io/badge/PostgreSQL-16-316192?logo=postgresql&logoColor=white)
![React](https://img.shields.io/badge/React-19-61dafb?logo=react&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-strict-3178c6?logo=typescript&logoColor=white)
![WebSockets](https://img.shields.io/badge/WebSockets-RFC%206455-4dabf7)
![license](https://img.shields.io/badge/license-All%20rights%20reserved-blue)

</div>

<p align="center">
  <img src="docs/screenshots/00-overview.png" alt="Live Chat — the room list on the left and a live conversation on the right (dark theme)" width="880">
</p>

---

## Table of contents

- [About](#about)
- [Capabilities](#capabilities)
- [Essential features](#essential-features)
- [Architecture](#architecture)
- [How to use](#how-to-use)
- [How to develop](#how-to-develop)
- [Tech stack](#tech-stack)
- [Author](#author)
- [License](#license)

## About

`live-chat-server` is an intermediate Rust project that demonstrates asynchronous
networking end to end: WebSockets for live messaging, PostgreSQL for durable
storage, JWT authentication, and room-based access control — paired with a
React + TypeScript client that speaks a typed protocol to the server.

Users can register, log in, create or join chat rooms, exchange messages in real
time, react to messages, and see full history after reconnecting. The system is
built **single-server first** (in-memory room registry + broadcast channels);
setting `REDIS_URL` switches it to cross-instance Pub/Sub broadcasting.

A few design properties carry through the whole codebase:

- **Save before broadcast** — every message is persisted to PostgreSQL before it
  is delivered over WebSockets.
- **Consistent identity** — messages carry IDs and timestamps, so clients render
  history identically after every reconnect.
- **Be defensive everywhere** — message-size limits, input validation, and
  membership re-checks apply to every action, not just the happy path.
- **Observable** — structured `tracing` logs for connects, disconnects, errors,
  and broadcasts; `/health` liveness probe.

Full HTTP + WebSocket protocol reference: **[`docs/api.md`](docs/api.md)**.

## Capabilities

**Backend — Rust, Axum + Tokio**

- Auth: register / login with **argon2id** hashing and **JWT (HS256, 24 h)**
  bearer tokens; `/api/me`.
- Rooms: create, list, get, join (idempotent); open enrollment — a room name is
  enough to join. Leaving stays on the API (`POST /api/rooms/{id}/leave`) but is
  no longer surfaced in the UI.
- History: paginated messages (`limit` + `before` cursor), newest first,
  members only.
- Real-time chat: WebSocket with pre-upgrade auth + access checks, per-room
  broadcast, save-before-broadcast persistence, presence join/leave events,
  revoked-membership teardown, reactions, typing events, and read receipts.
- Storage: SQLx with compile-time-checked queries against PostgreSQL;
  migrations bundled and applied on startup.
- Optional Redis Pub/Sub fan-out when `REDIS_URL` is set (graceful in-memory
  fallback otherwise).

**Frontend — React 19 + TypeScript (Vite)**

- Register / login with live username and password validation.
- Room list with search and unread-count badges; create and join rooms.
- Live chat: streaming messages, load-older pagination, reactions with an
  emoji picker, typing indicators, presence, and read receipts ("Seen by...").
- Resilient WebSocket client with reconnect + backoff; marks the room read when
  you return to the tab.
- A persisted light theme scoped to the conversation panel, the About page and
  the auth screen — the app shell (topbar + room list) stays dark — with a
  theme toggle on every screen. Plus a room-search shortcut (`Ctrl/Cmd + K`),
  toasts, desktop notifications, keyboard navigation, and screen-reader labels.
- Typed REST + WebSocket client with response normalization, 118 unit tests.

## Essential features

Every capture below comes from the same seeded two-user room (alice + bob), in
the dark theme. Each block highlights one part of the app end to end.

### Authentication

Register or log in with live username and password validation that runs before
the request is sent. Passwords are hashed with argon2id on the server and
sessions use a JWT bearer token. A theme toggle and the About link sit in the
top-right corner.

<img src="docs/screenshots/01-auth.png" alt="Register / log in screen" width="720">

### Rooms

Search the room list, create a room, or join an existing one with a single
click. Unread badges mark rooms with new messages, and each room shows who
created it.

<img src="docs/screenshots/02-rooms.png" alt="Room list with unread badges" width="720">

### Live chat & read receipts

Messages stream over a single WebSocket and are persisted to PostgreSQL
**before** they are broadcast, so they reappear after a reconnect. Sending
advances a per-user read cursor, which is why other members see the
"Seen by …" receipt under your last message.

<img src="docs/screenshots/03-chat.png" alt="Live chat with a Seen by bob read receipt" width="720">

### Reactions

Hover an incoming message to reveal the quick reaction bar. A reaction is stored
and broadcast like any other event, and reactions already on a message show
inline beside it.

<img src="docs/screenshots/04-reactions.png" alt="Hover reaction bar on an incoming message" width="720">

### Emoji picker

The composer has an emoji picker for dropping an emoji into the draft without
leaving the keyboard.

<img src="docs/screenshots/05-emoji-picker.png" alt="Composer emoji picker open" width="720">

### Typing & presence

The server relays ephemeral typing events, so the room shows who is typing, and
presence join/leave events keep the online list current.

<img src="docs/screenshots/06-typing.png" alt="Typing indicator showing bob is typing" width="720">

### About page

A hash-routed About page (`#/about`) with a quick how-to, the stack, and links
to the repo and the API reference. It is reachable logged in or out, and carries
its own theme toggle.

<img src="docs/screenshots/07-about.png" alt="About page" width="720">

## Architecture

```mermaid
flowchart LR
    B["Browser<br/>(React + TypeScript)"]
    B -->|"REST /api<br/>auth · rooms · history"| A["Rust API<br/>(Axum + Tokio)"]
    B -->|"WebSocket /api/ws<br/>live message stream"| A
    A -->|"save-before-broadcast"| DB[("PostgreSQL<br/>users · rooms · memberships · messages")]
    A -->|"in-memory room registry<br/>+ broadcast channels"| A
    A -.->|"REDIS_URL set →"| R[("optional Redis<br/>cross-instance Pub/Sub")]
```

The client only ever talks to the Vite dev server, which proxies `/api`
(HTTP + WebSocket) and `/health` to the backend — no CORS during development.
WebSocket frames are normalized by a typed client so the UI and the server
agree on optional fields even across server versions.

## How to use

The easiest way to see it running is Docker Compose:

```bash
make dev-up        # Postgres + API (:8083) + frontend (:5173)
```

1. Open <http://localhost:5173>.
2. Register a username and password (any unique username works).
3. Create a room — or ask a friend for a room name and join it.
4. Chat: messages, emoji reactions, typing indicators, read receipts.
5. Close the tab and come back: history is persisted in PostgreSQL — open the
   room again and the client fetches it via `GET /api/rooms/{id}/messages`.

Stop everything with `make dev-down`.

No Docker? Run the pieces directly — see [How to develop](#how-to-develop).

## How to develop

### Prerequisites

- Rust 1.99+ (the mise toolchain pins `rust`), and Node.js 24+ with npm —
  Node is not pinned by mise; use nvm, corepack, or your package manager
- Docker + Docker Compose for `make dev-up`, or a local PostgreSQL

### Quick start

```bash
make dev-up         # full stack with one command
```

Or run each component by hand — backend first (it runs migrations on startup):

```bash
cd backend
cargo fetch
cargo run           # reads DATABASE_URL + PORT (default 8080)
```

```bash
cd frontend
npm ci
npm run dev         # Vite on :5173, proxies /api to the backend
```

The Vite proxy forwards `/api` (HTTP + WebSocket) and `/health` to the
backend. It defaults to `http://localhost:8083`, which is where `make dev-up`
exposes the API. When running the backend by hand on its default port, point
the proxy at it:

```bash
cd frontend
VITE_API_PROXY_TARGET=http://localhost:8080 npm run dev
```

### Commands

| Command | What it does |
|---|---|
| `make check` | Every gate: backend fmt/clippy/test/build (`--locked`) + frontend format/lint/typecheck/test/build |
| `make dev-up` / `make dev-down` | Start / stop the Compose stack |
| `make db-clean` | Truncate all app data in the dev database (destructive) |
| `make screenshots` | Playwright capture harness (needs the stack running) |
| `cd backend && cargo test` | Backend unit + integration tests (WS, pub/sub, read receipts); needs a reachable `DATABASE_URL` |
| `cd backend && sqlx migrate run` | Apply migrations manually |
| `cd frontend && npm run test` | Frontend Vitest unit tests |
| `cd frontend && npm run build` | Type-check + production build |

Ports used by the Compose stack (override via `.env`):

| Service | Host | Container |
|---|---|---|
| API | 8083 | 8080 |
| Frontend | 5173 | 5173 |
| PostgreSQL | 5434 | 5432 |
| Redis (optional) | 6381 | 6379 |

### Regenerating the screenshots

The harness (`frontend/scripts/screenshots.mjs`) seeds a two-user room over the
public API and WebSocket — messages, reactions, a read receipt, typing — then
captures one PNG per feature plus the README hero (the eight images shown
above), all in the dark theme. It is not part of `make check` and CI never runs
it.

```bash
npm ci
npx playwright install chromium     # one-time; on Linux usually needs --with-deps (requires sudo)
make dev-up                          # stack must be running
SCREENSHOTS_DIR=../docs/screenshots make screenshots   # committed output
```

The default output directory is `frontend/screenshots/` (gitignored); the
published images live in `docs/screenshots/` (committed) — hence the
`SCREENSHOTS_DIR=../docs/screenshots` override above, relative to the
`frontend/` directory the harness runs in. `VITE_ORIGIN` points the browser
elsewhere if your stack is not on `:5173`.

## Tech stack

| Layer | Choice |
|---|---|
| Backend | Rust — **Axum 0.8** on Tokio, **SQLx 0.9** (compile-time-checked SQL), Serde, tracing |
| Database | **PostgreSQL 16**, migrations with sqlx |
| Realtime | WebSockets, per-room broadcast; optional **Redis Pub/Sub** for multi-instance |
| Auth | argon2id password hashing, **JWT (HS256)** bearer tokens |
| Frontend | **React 19** + TypeScript (Vite); Oxlint + Oxfmt; Vitest |
| Tooling | Docker Compose, mise, GitHub Actions CI (Rust + frontend jobs) |

## Author

Built by **[Felipe Inoue](https://github.com/fiwon123)** — a study and
portfolio project demonstrating asynchronous networking, WebSockets,
concurrency, authentication, and persistence. Feedback and questions are
welcome — open an issue or reach out via the GitHub profile.

## License

All rights reserved — this is a personal portfolio project, **not** open
source. See [LICENSE](LICENSE) for the full notice.

Third-party dependencies (Axum, Tokio, SQLx, React, Vite, and others) retain
their own licenses; see `backend/Cargo.lock` and `frontend/package-lock.json`.

---

<p align="center"><a href="#table-of-contents">Back to top</a></p>
