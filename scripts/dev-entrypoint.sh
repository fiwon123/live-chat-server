#!/usr/bin/env bash
# CRLF guard (#436)
if [[ -f "${BASH_SOURCE[0]}" ]] && grep -q $'\r$' "${BASH_SOURCE[0]}"; then # crlf-guard
    printf '%s\n' "[dev] ${BASH_SOURCE[0]##*/} has CRLF line endings; re-checkout with LF." >&2; exit 1 # crlf-guard
fi # crlf-guard
set -euo pipefail
DEV_DIR="${DEV_DIR:-/workspace}"
if ! getent passwd "$(id -u)" >/dev/null; then
  if [[ "$(id -u)" -eq 0 ]]; then printf 'sandbox:x:%s:%s:Sandbox User:/home/%s:/bin/bash\n' "$(id -u)" "$(id -g)" "${DEV_USER:-devuser}" >> /etc/passwd; fi
fi
if [ "${SLEEP_ON_IDLE:-true}" = "true" ] && [ ! -f "${DEV_DIR}/backend/Cargo.toml" ] && [ ! -f "${DEV_DIR}/backend/Cargo.lock" ]; then
  echo "[dev] No code found; sleeping indefinitely for shell access..."
  exec sleep infinity
fi
echo "[dev] Starting the API (cargo run)..."
cd "${DEV_DIR}" && exec cargo run --manifest-path backend/Cargo.toml
