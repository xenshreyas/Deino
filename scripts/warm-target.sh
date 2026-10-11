#!/usr/bin/env bash
# Run after each merge to main. It runs the gate's Rust steps at low priority,
# which both checks the merged code and warms the main checkout's `target/`.
# New agent worktrees clone that `target/` (scripts/dev-env.sh), so their first
# gate run only rebuilds the crates they touched. It takes the gate lock, so it
# never overlaps an agent's gate run.
set -euo pipefail
cd "$(dirname "$0")/.."
exec nice -n 15 scripts/ci-local.sh clippy test bindings sqlx
