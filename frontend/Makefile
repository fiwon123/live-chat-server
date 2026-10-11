# Frontend component Makefile fragment (consumed by root Makefile)
#
# Same principle as the backend fragment: the gate does NOT swallow failures —
# a check that cannot fail is worse than no check, because it is reported as
# passing.
#
# Dependencies are installed explicitly (`make install`, or `npm ci`) so the
# gate itself stays deterministic and offline. `check` fails loudly with a clear
# message when they are missing rather than skipping.

.PHONY: check install format lint typecheck build

check: ## format check + lint + typecheck + build (the gate)
	@command -v npm >/dev/null 2>&1 || { echo "ERROR: npm is not on PATH. Install Node 24+ and retry." >&2; exit 1; }
	@test -d node_modules || { echo "ERROR: frontend deps are missing. Run: cd frontend && npm ci" >&2; exit 1; }
	npm run format:check
	npm run lint
	npm run typecheck
	npm run build

install: ## Install dependencies from the lockfile
	npm ci

format: ## Rewrite files with oxfmt
	npm run format

lint: ## Run oxlint
	npm run lint

typecheck: ## Type-check only (tsc -b)
	npm run typecheck

build: ## Production build
	npm run build
