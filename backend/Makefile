# Rust component Makefile fragment (consumed by root Makefile)
#
# Unlike the other fragments, this one does NOT swallow failures with `|| true`.
# rustfmt, clippy, test, and build are the gate -- a check that cannot fail is
# worse than no check, because it is reported as passing.
#
# `--locked` is what makes the gate meaningful: it fails when Cargo.lock is
# missing or stale, so the check runs against exactly the reviewed dependency
# set (Cargo.lock is the lockfile).
#
# `SQLX_OFFLINE=true` resolves sqlx's compile-time query macros from the
# committed `.sqlx/` cache instead of a live database. That makes the gate
# deterministic and DB-independent (same as CI) and fails loudly when a query
# changed without regenerating the cache (`cargo sqlx prepare`). DB-backed
# integration tests still connect at runtime via DATABASE_URL.
.PHONY: check
check:
	@if ! command -v cargo >/dev/null 2>&1; then \
		echo "cargo not found; skipping Rust checks"; \
	elif [ ! -f Cargo.toml ]; then \
		echo "no Cargo.toml here yet -- Rust checks SKIPPED (skipped is not passed)"; \
	else \
		SQLX_OFFLINE=true cargo fmt --check; \
		SQLX_OFFLINE=true cargo clippy --locked --all-targets -- -D warnings; \
		SQLX_OFFLINE=true cargo test --locked; \
		SQLX_OFFLINE=true cargo build --locked; \
	fi
