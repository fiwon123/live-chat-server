# frontend

React + TypeScript single-page client for live-chat-server, built with Vite.

## Commands

```bash
npm ci            # install exactly the lockfile (required before make check)
npm run dev       # dev server (http://localhost:5173)
npm run build     # type-check + production build into dist/
npm run preview   # serve the production build
npm run typecheck # tsc -b
npm run test      # vitest run (unit tests)
npm run test:watch
npm run lint      # oxlint (--deny-warnings)
npm run lint:fix  # oxlint --fix
npm run format    # oxfmt (write)
npm run format:check
```

`make check` runs format check → lint → typecheck → test → build and fails on
the first problem. Dependencies must be installed first (`npm ci`).

## Development

The easiest way to run the whole stack is `make dev-up` from the repo root
(Docker Compose starts Postgres + the API + this dev server); the UI is then at
<http://localhost:5173>.

To run the dev server on its own host, the Vite proxy forwards `/api` (HTTP +
WebSocket) and `/health` to the backend so the browser only ever talks to the
Vite origin. Point it elsewhere with:

```bash
VITE_API_PROXY_TARGET=http://localhost:8083 npm run dev
```

Inside Compose this is set to `http://dev:8080` (the API service name).

Set `VITE_API_BASE` to call an absolute API base instead of the proxied `/api`.

## Toolchain

- **Build**: Vite + React + TypeScript
- **Format**: Oxfmt — do not add Prettier
- **Lint**: Oxlint — do not add ESLint

## Screenshots

The Playwright harness (`scripts/screenshots.mjs`) seeds a two-user room over
the public API + WebSocket — messages, reactions, and a read receipt — then
captures the key screens as PNGs for visual review:

- `01-auth-dark.png` / `02-auth-light.png` — login
- `03-rooms-dark.png` — room list with unread badge
- `04-chat-dark.png` / `06-chat-light.png` — live chat
- `05-chat-seen-dark.png` — "Seen by bob" receipt

It is **not** part of `make check` and CI never runs it.

It needs a running stack plus one-time browser install:

```bash
npm ci
npx playwright install chromium     # one-time; on Linux use --with-deps
make dev-up                          # or: backend cargo run + npm run dev
make screenshots                     # PNGs land in frontend/screenshots/
```

On Linux the browser usually needs system libraries before it will launch; if
it fails, run `npx playwright install --with-deps chromium` so the missing
packages are installed (requires sudo). On macOS/Windows the plain install
above is enough.

Override where the browser points (default `http://localhost:5173`, the Vite
origin — `/api` and `/health` are proxied) or where PNGs land:

```bash
VITE_ORIGIN=http://localhost:5173 SCREENSHOTS_DIR=/tmp/shots make screenshots
```

Capture settings (viewport, origin) mirror `playwright.config.ts`, which also
exists for future `e2e/` Playwright tests.

## API

The backend HTTP + WebSocket API is documented in
[`../docs/api.md`](../docs/api.md).
