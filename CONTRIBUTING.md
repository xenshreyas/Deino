# Contributing to Deino

Deino is a private repository. Work is organised as task cards in
[`docs/PARITY_PLAN.md`](docs/PARITY_PLAN.md) (§6), scheduled in waves (§7).

## Workflow

1. Pick a card whose dependencies are merged to `main`.
2. Work in its own branch, `feat/<CARD-ID>-<slug>`, ideally in a separate git worktree.
3. Follow the engineering rules in §3, §3A, §3B and §3C of the plan.
4. Use Conventional Commits (`feat(scope): …`, `fix(…)`, `refactor(…)`, `chore(…)`, `docs(…)`).
5. Before opening a pull request, make sure these pass:
   ```bash
   cd frontend && pnpm run build && bun test
   cd frontend/src-tauri && cargo build && cargo test
   ```
6. In the pull request, describe what changed, why, how it was tested (with
   screenshots for UI), which docs were updated, and any follow-ups.

Never push directly to `main`, and never push to the `upstream` remote.
