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

The dev server proxies `/api` (HTTP + WebSocket) and `/healthz` to the backend
so the browser only ever talks to the Vite origin. Point it elsewhere with:

```bash
VITE_API_PROXY_TARGET=http://localhost:8083 npm run dev
```

Set `VITE_API_BASE` to call an absolute API base instead of the proxied `/api`.

## Toolchain

- **Build**: Vite + React + TypeScript
- **Format**: Oxfmt — do not add Prettier
- **Lint**: Oxlint — do not add ESLint

## API

The backend HTTP + WebSocket API is documented in
[`../docs/api.md`](../docs/api.md).
