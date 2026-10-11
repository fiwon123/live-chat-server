# live-chat-server

An intermediate Rust project demonstrating asynchronous networking, WebSockets,
concurrent connections, authentication, and message persistence (Axum + Tokio +
SQLx + PostgreSQL). Single-server with in-memory room state; Redis Pub/Sub is
deferred to a multi-instance extension.

## Status

**MVP on `dev`.** Users can register, log in, create or join chat rooms, exchange
text messages in real time over WebSockets, and reload history after
reconnecting. A frontend is planned (React + Vite + TS). See
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
  broadcasts); `/healthz` liveness probe.

Full endpoint and WebSocket protocol reference: **[`docs/api.md`](docs/api.md)**.

## Stack

- **Backend**: Rust (Axum on Tokio), SQLx, Serde, tracing
- **Database**: PostgreSQL 16
- **Dev**: Docker Compose + mise
- **Frontend**: React + Vite + TS + Oxfmt + Oxlint (planned)

## Layout

```text
backend/    # API + WS + persistence + auth (routes, services, storage, ws)
docs/       # api.md — endpoint and WebSocket protocol reference
```

## Development

```bash
make check        # fmt + clippy + test + build (backend; locked)
make dev-up       # Postgres (+ Redis commented), dev sandbox
make shell        # inside sandbox
make opencode     # opencode inside sandbox
```

```bash
cd backend
cargo run         # reads DATABASE_URL + PORT (default 8080); applies migrations on start
cargo test
```

Copy `.env.example` to `.env` for local runs.

## Workflow

Issue → branch → PR → merge on `main` ← `dev` ← feature branches. CI runs only on
`dev` → `main` PRs; local `make check` is the feature-branch gate.
- `AGENTS.md` — agent rules and conventions (save-before-broadcast, IDs+timestamps, limits/validation)
- `WORKFLOW_PROMPT.md` — dev workflow prompt
- `.opencode/instructions/` — workflow, testing, memory