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

## API

The backend HTTP + WebSocket API is documented in
[`../docs/api.md`](../docs/api.md).
