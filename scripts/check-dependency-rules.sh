#!/usr/bin/env bash
# PARITY_PLAN §4.0 dependency rules, checked on the resolved graph:
#
#   - no crate under crates/ depends on `tauri` (the core stays Tauri-free);
#   - `deino-stt` never depends on `deino-net` or an HTTP client (audio and
#     transcripts never leave the Mac, so the STT crate cannot reach the network).
#
# Every target is included, so a macOS-only dependency fails on Linux too.
# Normal, build and dev edges count: a test that needs Tauri belongs in the app.
#
#   scripts/check-dependency-rules.sh
set -euo pipefail

cd "$(dirname "$0")/.."

failures=0

# Every package that `crate` depends on, one `name` per line.
dependencies_of() {
    cargo tree --locked --package "$1" --target all --edges normal,build,dev \
        --prefix none --format '{p}' | awk '{ print $1 }' | sort -u
}

# Fails if `crate` depends on any of the named packages.
forbid() {
    local crate="$1" deps
    shift
    deps="$(dependencies_of "$crate")"
    for banned in "$@"; do
        if grep -qxF "$banned" <<<"$deps"; then
            echo "error: $crate depends on $banned (PARITY_PLAN §4.0); see: cargo tree -p $crate -i $banned --target all" >&2
            failures=$((failures + 1))
        fi
    done
}

# The package name each crates/*/Cargo.toml declares.
crate_names() {
    for manifest in crates/*/Cargo.toml; do
        awk -F'"' '/^name = / { print $2; exit }' "$manifest"
    done
}

checked=0
for crate in $(crate_names); do
    forbid "$crate" tauri
    checked=$((checked + 1))
done

summary="$checked crates are Tauri-free"
if [[ -f crates/deino-stt/Cargo.toml ]]; then
    forbid deino-stt deino-net reqwest hyper ureq
    summary="$summary; deino-stt has no network client"
fi

if ((failures > 0)); then
    exit 1
fi
echo "Dependency rules OK ($summary)"
