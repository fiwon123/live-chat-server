# live-chat-server

An intermediate Rust project demonstrating asynchronous networking, WebSockets,
concurrent connections, authentication, and message persistence (Axum + Tokio +
SQLx + PostgreSQL). Single-server with in-memory room state; Redis Pub/Sub is
deferred to a multi-instance extension.

## Status

**Scaffold only.** Workflow configuration bootsrapped; no application code yet.
See `PROJECT_CONTEXT.md` and `AGENTS.md`.

## Stack

- **Backend**: Rust (Axum on Tokio), SQLx, Serde, tracing
- **Database**: PostgreSQL 16
- **Dev**: Docker Compose + mise
- **Frontend**: React + Vite + TS + Oxfmt + Oxlint (deferred)

## Layout

```text
backend/    # API + WS + persistence + auth (to be added)
```

## Development

```bash
make check        # fmt + clippy + test + build (skipped until Cargo.toml exists)
make dev-up       # Postgres (+ Redis commented), dev sandbox
make shell        # inside sandbox
make opencode     # opencode inside sandbox
```

Copy `.env.example` to `.env` for local runs.

## Workflow

Issue → branch → PR → merge on `main` ← `dev` ← feature branches. CI runs only on
`dev` → `main` PRs; local `make check` is the feature-branch gate.
- `AGENTS.md` — agent rules and conventions (save-before-broadcast, IDs+timestamps, limits/validation)
- `WORKFLOW_PROMPT.md` — dev workflow prompt
- `.opencode/instructions/` — workflow, testing, memory
