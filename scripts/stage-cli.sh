#!/usr/bin/env bash
# Builds the `deino` CLI (crate deino-cli) and stages it as the app's `cli`
# sidecar, frontend/src-tauri/binaries/cli-<target triple>, which Tauri
# bundles as Deino.app/Contents/MacOS/cli (DEV-02/04, ADR 0033).
#
#   scripts/stage-cli.sh                       # debug build, for `pnpm tauri:dev`
#   scripts/stage-cli.sh --release             # release build, host target
#   scripts/stage-cli.sh --release --target aarch64-apple-darwin
#
# A dev build then finds the CLI beside its own executable
# (target/debug/cli, copied there by tauri-build), so Settings → Developer
# can install it.
set -euo pipefail

cd "$(dirname "$0")/.."

profile=debug
target=""
args=()
while (($# > 0)); do
    case "$1" in
        --release) profile=release && args+=(--release) ;;
        --target) target="$2" && args+=(--target "$2") && shift ;;
        *) echo "usage: $0 [--release] [--target <triple>]" >&2 && exit 2 ;;
    esac
    shift
done

triple="${target:-$(rustc -vV | sed -n 's/^host: //p')}"
cargo build --locked -p deino-cli --bin deino-cli ${args[@]+"${args[@]}"}
built="target/${target:+$target/}$profile/deino-cli"
staged="frontend/src-tauri/binaries/cli-$triple"
mkdir -p "$(dirname "$staged")"
cp "$built" "$staged"
echo "staged $built as $staged"
