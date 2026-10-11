# live-chat-server

An intermediate Rust project demonstrating asynchronous networking, WebSockets,
concurrent connections, authentication, and message persistence (Axum + Tokio +
SQLx + PostgreSQL). Single-server with in-memory room state; Redis Pub/Sub is
deferred to a multi-instance extension.

## Status

**MVP on `dev`.** Users can register, log in, create or join chat rooms, exchange
text messages in real time over WebSockets, and reload history after
reconnecting. A React + Vite + TypeScript client ships alongside the backend. See
`PROJECT_CONTEXT.md` and `AGENTS.md` for the brief and agent rules.

### What is implemented

- **Auth**: register / login with argon2id password hashing; JWT (HS256, 24 h)
  bearer tokens; `/api/me`.
- **Rooms**: create, list, get, join/leave (idempotent); open join.
- **History**: paginated message history (`limit` + `before` cursor), newest
  first, members only.
- **Real-time chat**: WebSocket endpoint with pre-upgrade auth + access checks,
  per-room broadcast, save-before-broadcast persistence (messages carry IDs and
  timestamps), per-message validation and membership re-checks, presence
  join/leave events, revoked-membership teardown.
- **Observability**: structured `tracing` logs (connect/disconnect, errors,
  broadcasts); `/health` liveness probe.
- **Frontend**: React 19 + Vite + TS SPA — register/login, room
  list/create/join, live chat (history, presence, reconnect with backoff), a
  typed REST + WebSocket client, and Vitest unit tests. The dev server proxies
  `/api` (HTTP + WS) to the backend, so there is no CORS.

Full endpoint and WebSocket protocol reference: **[`docs/api.md`](docs/api.md)**.

## Stack

- **Backend**: Rust (Axum on Tokio), SQLx, Serde, tracing
- **Database**: PostgreSQL 16
- **Frontend**: React 19 + Vite + TS; Oxlint (lint) + Oxfmt (format); npm
- **Dev**: Docker Compose + mise

## Layout

```text
backend/    # API + WS + persistence + auth (routes, handlers, ws, services, storage, models)
frontend/   # React + Vite + TS client (api/, chat/, hooks/, components/)
docs/       # api.md — endpoint and WebSocket protocol reference
```

## Development

`make check` runs every component's gate (format, lint, test, build) and fails
on the first problem. Install dependencies first: `cargo fetch` (backend) and
`npm ci` (from `frontend/`).

```bash
make check        # backend (fmt + clippy + test + build, --locked) + frontend gate
make dev-up       # Postgres + API (:8083) + frontend dev server (:5173)
make shell        # inside sandbox
make opencode     # opencode inside sandbox
```

`make dev-up` (Docker Compose) starts everything for development — the UI at
<http://localhost:5173>, the API at <http://localhost:8083>, Postgres published
on `5434`. The Vite dev server proxies `/api` (HTTP + WebSocket) and `/health`
to the API container, so there is no CORS. All ports are overridable via `.env`
(`HOST_PORT`, `FRONTEND_HOST_PORT`, `DB_HOST_PORT`).

Without Docker, run each component directly:

```bash
cd backend && cargo run    # reads DATABASE_URL + PORT (default 8080); runs migrations on start
cd backend && cargo test   # unit + integration tests
```

```bash
cd frontend && npm ci && npm run dev   # Vite dev server on :5173, proxies /api to the backend
cd frontend && npm run build           # type-check + production build
cd frontend && npm run test            # Vitest unit tests
```

Copy `.env.example` to `.env` for local runs (Docker Compose reads it).

## Workflow

Issue → branch → PR → merge on `main` ← `dev` ← feature branches. CI (Rust +
frontend jobs) runs only on `dev` → `main` PRs; local `make check` is the
feature-branch gate.
- `AGENTS.md` — agent rules and conventions (save-before-broadcast, IDs+timestamps, limits/validation)
- `WORKFLOW_PROMPT.md` — dev workflow prompt
- `.opencode/instructions/` — workflow, testing, memory