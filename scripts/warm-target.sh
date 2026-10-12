#!/usr/bin/env bash
# Run after each merge to main. It compiles what the gate compiles (clippy and
# the test binaries) into the main checkout's `target/`, without running tests.
# New agent worktrees clone that `target/` (scripts/dev-env.sh), so their first
# gate run only rebuilds the crates they touched.
#
# It never takes the gate lock, so it can't delay an agent's gate. Instead it
# stays out of the way: lowest CPU priority and two compile jobs.
set -uo pipefail
cd "$(dirname "$0")/.." || exit
export CARGO_BUILD_JOBS=2 DEINO_GATE_NO_LOCK=1
nice -n 19 scripts/ci-local.sh clippy || echo "warm-target: clippy reported problems on main" >&2
# shellcheck source=scripts/dev-env.sh
source scripts/dev-env.sh
nice -n 19 cargo nextest run --workspace --locked --no-run --cargo-quiet
