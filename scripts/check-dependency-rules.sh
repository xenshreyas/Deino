#!/usr/bin/env bash
# PARITY_PLAN §4.0 dependency rules, checked on the resolved graph:
#
#   - no crate under crates/ depends on `tauri` (the core stays Tauri-free);
#   - `deino-audio`, `deino-detect` and `deino-stt` never depend, even
#     indirectly, on `deino-net` or an HTTP client (audio, transcripts and
#     what runs on the Mac never leave it, so those crates cannot reach the
#     network; FND-12, CAP-06);
#   - no crate under crates/ except `deino-net` depends directly on an HTTP
#     client (FND-12: deino-net owns the only one; the app crate's inherited
#     LLM clients are the exception until FND-09, and clippy's
#     `disallowed-types` covers the app's new code).
#
# Every target is included, so a macOS-only dependency fails on Linux too.
# Normal, build and dev edges count for the first two rules: a test that
# needs Tauri belongs in the app. The direct-client rule counts normal and
# build edges only, so a test may talk to a local server.
#
#   scripts/check-dependency-rules.sh
set -euo pipefail

cd "$(dirname "$0")/.."

# The crates that must have no route to the network.
readonly OFFLINE_CRATES=(deino-audio deino-detect deino-stt)
# Every HTTP client crate a dependency could bring in.
readonly HTTP_CLIENTS=(reqwest hyper ureq isahc surf attohttpc curl)

failures=0

# Every package that `crate` depends on, one `name` per line.
dependencies_of() {
    cargo tree --locked --package "$1" --target all --edges normal,build,dev \
        --prefix none --format '{p}' | awk '{ print $1 }' | sort -u
}

# The packages `crate` depends on directly through normal and build edges.
direct_dependencies_of() {
    cargo tree --locked --package "$1" --target all --edges normal,build \
        --depth 1 --prefix none --format '{p}' | tail -n +2 | awk '{ print $1 }' | sort -u
}

# Fails if `crate` depends on any of the named packages; `$1` lists them.
forbid_in() {
    local deps="$1" crate="$2"
    shift 2
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
    forbid_in "$(dependencies_of "$crate")" "$crate" tauri
    if [[ "$crate" != deino-net ]]; then
        forbid_in "$(direct_dependencies_of "$crate")" "$crate" "${HTTP_CLIENTS[@]}"
    fi
    checked=$((checked + 1))
done

offline=()
for crate in "${OFFLINE_CRATES[@]}"; do
    [[ -f "crates/$crate/Cargo.toml" ]] || continue
    forbid_in "$(dependencies_of "$crate")" "$crate" deino-net "${HTTP_CLIENTS[@]}"
    offline+=("$crate")
done

if ((failures > 0)); then
    exit 1
fi
echo "Dependency rules OK ($checked crates are Tauri-free; only deino-net has an HTTP client;" \
    "${offline[*]:-no offline crates yet} cannot reach the network)"
