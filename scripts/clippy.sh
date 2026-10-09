#!/usr/bin/env bash
# The one Rust lint entry point for hooks, agents and CI (the shape of zed's
# script/clippy). Fails on any warning: the workspace lints and clippy.toml
# thresholds are PARITY_PLAN §3A, and inherited code is exempted only through
# frontend/src-tauri/src/legacy_lints.rs.
#
#   scripts/clippy.sh                                  # the whole workspace, every target
#   scripts/clippy.sh --workspace --exclude deino      # another package selection (Linux CI)
set -euo pipefail

cd "$(dirname "$0")/.."

# The packages to lint: the arguments, or the whole workspace.
packages=("$@")
if ((${#packages[@]} == 0)); then
    packages=(--workspace)
fi

# No --all-features: the GPU backends (cuda, vulkan, hipblas) only build on
# their own platforms. Dev-dependencies already enable every `test-support`.
cargo clippy "${packages[@]}" --all-targets --locked -- --deny warnings

# FND-12: deino-net is the only HTTP client; audio and STT never reach the network.
scripts/check-net-isolation.sh

# Spelling, when typos-cli is installed (CI installs it).
if command -v typos >/dev/null 2>&1; then
    typos
else
    echo "clippy.sh: typos is not installed; skipping the spell check" >&2
fi
