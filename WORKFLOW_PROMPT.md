# Development Workflow Prompt

You are the primary AI agent for `live-chat-server`. Drive development work
using this workflow. Every change is tracked end-to-end on GitHub:
**issue → branch → PR → merge** with issue and PR assigned, labeled, linked, and
commented at each milestone.

## 1. Plan and understand

- Understand requirement; ask clarifying questions if ambiguous.
- Explore codebase (read relevant files, follow existing patterns).
- **Create or understand the GitHub issue**: ensure it exists, assigned (`--assignee fiwon123`), labeled per commit type, milestone (`v0.1-MVP`), and post "Starting work on this — Milestone: v0.1-MVP".
- Output: Goal, Files to modify, Dependencies, Risks, Steps. Get user approval before implementation.

## 2. Branch

```bash
git checkout dev
git pull origin dev
git checkout -b <type>/<issue-number>-<slug>
```
Branch types: `feat/`, `fix/`, `refactor/`, `docs/`, `test/`, `chore/`, `ci/`

## 3. Implement

- Small, focused commits with conventional commits
- Rust: `cargo fmt` changed files; keep clippy clean (warnings denied)
- **Save before broadcast**: persist messages before WS broadcast
- IDs + timestamps for consistency across reconnects
- Enforce message-size limits, input validation, room-access checks
- In-memory room registry (single-instance); Redis later
- Do NOT create issues/PRs from subagents

## 4. Test

- `make check` (fmt + clippy + test + build). **Until `backend/Cargo.toml` exists, steps are skipped — skipped is not passed.**
- Unit tests must not require Postgres/Redis
- DB/WS integration tests run against compose services when needed
- Two-client WS test for room isolation (the automated form of "two browser tabs")
- CI does not run on feature branches — local testing is your gate

## 5. Push & PR

```bash
git push origin <branch>
gh pr create --base dev --title "<type>: <description>" \
  --body "Closes #<issue-number>" --label <label> --assignee fiwon123 \
  --milestone "v0.1-MVP"
```
Comment on issue: `PR opened: #<pr-number>`

## 6. Review & Merge

- Self-review; merge only when explicitly instructed; squash merge, delete branch
- After merge to `dev`: comment on issue "Merged into dev — ready for release", comment on PR "Merged into dev — thanks!", **close issue explicitly** (GH auto-closes only into default branch; feature PRs go to `dev`)
- Return to `dev` after merge

## 7. Release (dev → main) — user-initiated

- User must explicitly ask; create `dev` → `main` PR with label `release`; CI runs on this PR
- Never use `--delete-branch` on release merge; after CI passes and user confirms, merge and then merge `main` back into `dev`; verify `dev` survived

## 8. Cleanup

- Return to `dev`, pull, prune

## Important

- Skipped is not passed
- No Docker socket inside sandbox
- Never commit secrets; `.sqlx/` must remain committed
