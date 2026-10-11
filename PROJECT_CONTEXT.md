# PROJECT_CONTEXT.md

> Structured project identity. `AGENTS.md` is populated from this file — keep the
> two in sync. This is the brief; `AGENTS.md` is the operating manual the agents
> read first.

## Identity

- **Project**: live-chat-server
- **One sentence**: An intermediate Rust project that demonstrates asynchronous
  networking, WebSockets, concurrent connections, authentication, and message
  persistence.

## Problem

Building a real-time chat system requires coordinating WebSocket connections,
enforcing room access and message validation, persisting a history of
conversations, and handling reconnects — all while keeping the system simple
enough to test and reason about with a single-server setup first.

## Goal

Users can register, log in, create or join chat rooms, exchange text messages in
real time, and view message history after reconnecting. The backend manages
users, rooms, access permissions, WebSocket connections, message validation,
storage, and broadcasting. A React + TypeScript frontend provides the UI.

## Stack

- **Backend**: Rust (Axum on Tokio)
- **Networking**: WebSockets (axum/ws or tungstenite), `tracing` for structured logs
- **Database**: PostgreSQL
- **Query layer**: sqlx (compile-time-checked SQL)
- **Serialization**: Serde
- **Frontend**: React + TypeScript (Vite), Oxlint + Oxfmt, npm
- **Dev environment**: Docker Compose + `mise` toolchain pins
- **Later**: Redis Pub/Sub for multi-instance broadcasting (commented/optional)

## Topology

Components:
- kind: backend, path: backend, purpose: HTTP API, auth, WebSocket server, room
  registry (in-memory, single-instance), persistence, and broadcasting
- kind: frontend, path: frontend, purpose: React + TypeScript SPA (Vite) for
  register/login, room management, and real-time messaging over WebSocket

## Architecture

```text
Client (browser)
  ↓  HTTP (auth/register/login) + WebSocket (/ws)
Rust API (Axum + Tokio)
  ├─→ PostgreSQL   (users, rooms, room_memberships, messages)
  └─→ In-memory    (per-room WS connection registry + broadcast channels)
```

## Directory Layout

```text
backend/    # Rust API: routes, handlers, ws, services, storage, models
frontend/   # React + TypeScript client (Vite): pages, components, API/WS client
```

## Key Commands

- **Install**: backend `cargo fetch`; frontend `npm ci` (from `frontend/`)
- **Dev server**: backend `cargo run` (reads `DATABASE_URL`, `PORT`); frontend `npm run dev`
- **Tests**: backend `cargo test` (unit + integration; DB-backed tests may skip if DB unavailable)
- **Lint**: backend `cargo clippy --all-targets -- -D warnings`; frontend `npm run lint`
- **Format**: backend `cargo fmt` / `cargo fmt --check`; frontend `npm run format` / `format:check`
- **Build**: backend `cargo build`; frontend `npm run build`
- **Migrations**: backend `sqlx migrate run`
- **All checks**: `make check` (delegates to each component's Makefile)

## Notes

- `make check` delegates to each component's Makefile. A component whose
  toolchain is absent is skipped — a skipped run is not a passed run.
- Single-server instance to start: live room connections and broadcast channels
  stay in memory. Redis Pub/Sub is only for multi-instance; the compose Redis
  block is left commented/optional.
- WebSockets: save each message to PostgreSQL **before broadcasting** it;
  messages must carry IDs and timestamps so clients display them consistently
  across reconnects.
- Frontend is configured (React + Vite + TS + Oxfmt + Oxlint); its own issue
  covers the chat UI.
- `.sqlx/` must **not** be gitignored — it's the offline query cache so
  `cargo build` doesn't require a live DB when using `SQLX_OFFLINE=true`.
- Docker dev sandbox: Postgres on host `5434`, Redis on host `6381` (if enabled),
  API on host `8083` by default — all overridable to avoid collisions.
