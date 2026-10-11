# frontend

React + TypeScript single-page client for live-chat-server, built with Vite.

## Commands

```bash
npm ci            # install exactly the lockfile (required before make check)
npm run dev       # dev server (http://localhost:5173)
npm run build     # type-check + production build into dist/
npm run preview   # serve the production build
npm run typecheck # tsc -b
npm run lint      # oxlint (--deny-warnings)
npm run lint:fix  # oxlint --fix
npm run format    # oxfmt (write)
npm run format:check
```

`make check` runs format check → lint → typecheck → build and fails on the
first problem. Dependencies must be installed first (`npm ci`).

## Toolchain

- **Build**: Vite + React + TypeScript
- **Format**: Oxfmt — do not add Prettier
- **Lint**: Oxlint — do not add ESLint

## API

The backend HTTP + WebSocket API is documented in
[`../docs/api.md`](../docs/api.md).
