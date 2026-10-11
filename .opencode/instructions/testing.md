## Testing Requirements

### General

- Tests mandatory for new features and bug fixes
- Write tests before or alongside implementation
- Mock external services when possible
- Aim for critical-path coverage, not 100%

### Prerequisites / Services

- DB-backed integration tests need Postgres: `docker compose up -d db` (or full stack)
- Unit tests must run without services; put DB tests behind a gate or in `tests/` that can skip if DB unavailable

### Backend

- **Framework**: built-in `cargo test`; `tokio::test` for async; `sqlx::test` for DB
- **Locations**: unit tests in `backend/src/` (`#[cfg(test)] mod tests`), integration in `backend/tests/`
- **Run**: `cargo test` (add `--locked` in CI)

#### Conventions

- Unit tests next to code; integration in `tests/` using crate as library
- Async: `#[tokio::test]`; DB tests: skip gracefully when DB unreachable
- Names describe behavior (e.g. `rejects_message_too_long`, `isolates_rooms`)
- Fixtures/helpers in `tests/common/` if shared
- **WebSocket integration**: spawn the app on an ephemeral port (e.g. with `axum::Server::bind` in a background task), open two WS clients in the same room and a third in another room, assert messages are delivered to the room but not cross-room. This is the automated form of "test with two browser tabs".

### Linting

```bash
cd backend
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
```

### Before Committing

1. `cargo fmt`
2. `cargo clippy --all-targets --locked -- -D warnings`
3. `cargo test --locked`
4. Or `make check`

## Visual checks

N/A until the React/Vite frontend lands. When it does, follow: **main agent captures, visual subagent looks, main agent acts**; never report findings the visual subagent hasn't returned.
