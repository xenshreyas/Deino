#!/usr/bin/env bash
# The one Rust lint entry point for hooks, agents and CI (the shape of zed's
# script/clippy). Fails on any warning: the workspace lints and clippy.toml
# thresholds are PARITY_PLAN §3A, and inherited code is exempted only through
# frontend/src-tauri/src/legacy_lints.rs.
#
#   scripts/clippy.sh            # lint the whole workspace, every target
set -euo pipefail

cd "$(dirname "$0")/.."

# No --all-features: the GPU backends (cuda, vulkan, hipblas) only build on
# their own platforms. Dev-dependencies already enable every `test-support`.
cargo clippy --workspace --all-targets --locked -- --deny warnings

# Spelling, when typos-cli is installed (CI installs it).
if command -v typos >/dev/null 2>&1; then
    typos
else
    echo "clippy.sh: typos is not installed; skipping the spell check" >&2
fi
