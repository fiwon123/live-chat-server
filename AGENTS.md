# AGENTS.md

> Source: Populated from `PROJECT_CONTEXT.md`. Keep them in sync.

### Product

## Project Objective

live-chat-server — an intermediate Rust project demonstrating asynchronous
networking, WebSockets, concurrent connections, authentication, and message
persistence.

### Problem

Coordinating WebSocket connections while enforcing room access and message
validation, persisting a history of conversations, and handling reconnects
becomes messy without clear boundaries. Starting with a single server and
in-memory room state keeps it testable; persistence and identity (IDs +
timestamps) keep it consistent.

### Goal

Users can register, log in, create or join chat rooms, exchange text messages in
real time, and view message history after reconnecting. The backend manages
users, rooms, access permissions, WebSocket connections, message validation,
storage, and broadcasting.

### Main Technology

- Backend: Rust (Axum on Tokio)
- Networking: WebSockets, `tracing` for structured logging
- Database: PostgreSQL
- Query layer: sqlx (compile-time-checked SQL)
- Serialization: Serde
- Frontend: React + TypeScript (Vite), Oxlint + Oxfmt, npm
- Development: Docker Compose (optional) + `mise` toolchain pins
- Later: Redis Pub/Sub for multi-instance broadcasting

### Core Architecture

```text
Client (browser)
  ↓  HTTP (auth/register/login) + WebSocket (/ws)
Rust API (Axum + Tokio)
  ├─→ PostgreSQL   (users, rooms, room_memberships, messages)
  └─→ In-memory    (per-room WS connection registry + broadcast channels)
```

## Project Structure

```text
backend/    # Rust API: routes, handlers, ws, services, storage, models
frontend/   # React + TypeScript client (Vite): pages, components, API/WS client
```

## Development Commands

### Backend

```bash
cd backend
cargo build
cargo run
sqlx migrate run
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

### Frontend

```bash
cd frontend
npm ci
npm run dev
npm run build
npm run typecheck
npm run lint
npm run format
```

### Docker Compose (optional)

```bash
docker compose up -d
docker compose down
docker compose logs -f dev
```

### Run All Checks

```bash
make check
```

`make check` delegates to each component's Makefile.
- `backend/Makefile`: `cargo fmt --check`, `cargo clippy --locked
  --all-targets -- -D warnings`, `cargo test --locked`, `cargo build --locked`.
- `frontend/Makefile`: `npm run format:check`, `npm run lint`, `npm run
  typecheck`, `npm run test`, `npm run build` (install deps first with `npm ci`).

A component whose toolchain is absent is **skipped** — a skipped run is not a
passed run. A component whose toolchain is present but dependencies are missing
**fails** with instructions (`npm ci`, `cargo fetch`) rather than passing.

## Runtime Environment

The agent runs at the workspace root. Docker is not available inside the sandbox.

### What the agent CAN do

- Edit Markdown, JSON, YAML, Rust files
- Run git and GitHub CLI commands
- Run `cargo`, `rustfmt`, `clippy`, `sqlx` (once `backend/Cargo.toml` exists)
- Create/manage issues, PRs, milestones
- Validate configs

### What the agent CANNOT do

- Run `docker` or `docker compose` (no Docker socket in sandbox)
- Access Docker socket
- Modify host filesystem beyond workspace

### Ports

| Service | Host | Container |
|---------|------|-----------|
| dev API | 8083 | 8080      |
| Postgres| 5434 | 5432      |
| Redis   | 6381 | 6379      |

(Redis block is commented/optional; host ports overridable.)

## Key Conventions

- **Assignee**: `fiwon123`
- **Two-tier branches**: `main` (release only) ← `dev` (integration) ← feature branches
- **Issue-first**: no code without an issue; link PRs with `Closes #<number>`
- **Conventional commits**: `feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`, `ci:`
- **Save before broadcast**: persist each message to PostgreSQL before broadcasting over WebSockets
- **Consistent identity**: messages carry IDs and timestamps so clients display them consistently after reconnects; use `timestamptz` and stable ordering
- **Message validation**: enforce message-size limits, input validation, and room-access checks on every action
- **Single-server first**: room connections and broadcast channels stay in-memory; Redis Pub/Sub only for multi-instance (later)
- **Tracing**: use `tracing` for structured logs (connect/disconnect/errors/broadcasts)
- **sqlx offline cache**: `.sqlx/` must NOT be gitignored; use `SQLX_OFFLINE=true` where needed
- **Lockfile discipline**: all cargo commands use `--locked` in CI
- **Release-only CI**: runs on `dev` → `main` PRs + manual dispatch; feature branches never trigger CI — local `make check` is the gate; **skipped is not passed**

## Environment and Secret-File Rules

- `.env` is gitignored; read `.env.example` with Read tool only
- Never access `.env`, `.env.*`, `.envrc` without explicit permission
- Never print/dump env (`printenv`, `env`, `docker compose config`, etc.)
- Prefer checking existence; never commit secrets/tokens

## Agents

- **build** (primary): full development with write access to repo
- **plan** (primary): analysis/planning, read-only
- **reviewer** (subagent): code review, read-only
- **backend** (subagent): implements `backend/` only
- **tester** (subagent): writes/runs tests, scoped to test files
- **frontend** (subagent): implements `frontend/` only; React + Vite + TS, Oxlint/Oxfmt
- **visual** (subagent): reviews captured UI screenshots, read-only

Keep this list in sync with `.opencode/agents/` and the `agent` block in
`opencode.json`.

## CI

Runs on `dev` → `main` PRs + `workflow_dispatch`. Never on feature branches.
Paths filter `backend/**` and `frontend/**`. Jobs:
- **rust**: `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D
  warnings`, `cargo test --locked`, `cargo build --locked`.
- **frontend**: Node 24, `npm ci`, `format:check`, `lint`, `typecheck`, `build`.

**Skipped is not passed.**
