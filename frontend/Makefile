# Frontend component Makefile fragment (consumed by root Makefile)
#
# Same principle as the backend fragment: the gate does NOT swallow failures —
# a check that cannot fail is worse than no check, because it is reported as
# passing.
#
# Dependencies are installed explicitly (`make install`, or `npm ci`) so the
# gate itself stays deterministic and offline. A missing toolchain is skipped
# (like the backend); missing dependencies fail loudly with instructions.

.PHONY: check install format lint typecheck build screenshots

check: ## format check + lint + typecheck + build (the gate)
	@if ! command -v npm >/dev/null 2>&1; then \
		echo "npm not found; skipping frontend checks"; \
	elif [ ! -d node_modules ]; then \
		echo "ERROR: frontend dependencies not installed -- run 'npm ci' in frontend/" >&2; \
		exit 1; \
	else \
		npm run format:check && \
		npm run lint && \
		npm run typecheck && \
		npm run test && \
		npm run build; \
	fi

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

screenshots: ## Capture UI screenshots for visual review (needs dev servers)
	@if ! command -v npm >/dev/null 2>&1; then \
		echo "npm not found; skipping screenshots"; \
	elif [ ! -d node_modules ]; then \
		echo "ERROR: frontend dependencies not installed -- run 'npm ci' in frontend/" >&2; \
		exit 1; \
	else \
		npm run screenshots; \
	fi
