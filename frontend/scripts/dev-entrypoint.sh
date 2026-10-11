#!/bin/sh
# Dev entrypoint for the frontend container.
#
# node_modules lives in a named volume so it survives restarts but never leaks
# platform-specific binaries from the host. Docker seeds a named volume from the
# image only when it is empty, so a stale volume could otherwise shadow freshly
# resolved dependencies; reinstall whenever the lockfile hash changes.
set -eu

if [ ! -f package-lock.json ]; then
  echo "[frontend] package-lock.json not found — is the source mounted?" >&2
  exit 1
fi

lock_hash_file="node_modules/.lock-hash"
# Avoid a pipeline so a sha256sum failure is not masked by the last command's
# status (POSIX sh has no pipefail).
want_hash="$(sha256sum package-lock.json)"
want_hash="${want_hash%% *}"

have_hash=""
if [ -f "$lock_hash_file" ]; then
  have_hash="$(cat "$lock_hash_file")"
fi

if [ "$want_hash" != "$have_hash" ]; then
  echo "[frontend] installing dependencies (npm ci)..."
  npm ci
  printf '%s' "$want_hash" > "$lock_hash_file"
fi

exec npm run dev -- --host 0.0.0.0 --port 5173 --strictPort
