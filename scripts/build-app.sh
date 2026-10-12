#!/usr/bin/env bash
# Builds the release Deino.app with its real sidecars and signs it ad hoc.
#
#   scripts/build-app.sh             # build into dist-app/Deino.app
#   scripts/build-app.sh --install   # ...then replace /Applications/Deino.app
#
# Why not just `pnpm run tauri:build`: this repository lives under
# ~/Documents, which iCloud manages, and the files it builds carry extended
# attributes that make `codesign` fail ("resource fork, Finder information,
# or similar detritus not allowed"). Tauri's own signing step therefore fails;
# this script copies the bundle out without those attributes and signs the copy
# with the app's entitlements, the same way Tauri would.
set -euo pipefail
cd "$(dirname "$0")/.."
# shellcheck source=scripts/dev-env.sh
source scripts/dev-env.sh

readonly OUT="dist-app"
readonly BUNDLE="target/release/bundle/macos/Deino.app"
readonly ENTITLEMENTS="frontend/src-tauri/entitlements.plist"
TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
readonly TRIPLE

# The local LLM and the `deino` CLI, built for real (not the gate's placeholders).
stage_sidecars() {
    cargo build -q --release -p llama-helper --features metal
    cp target/release/llama-helper "frontend/src-tauri/binaries/llama-helper-$TRIPLE"
    scripts/stage-cli.sh --release >/dev/null
}

# Tauri's build; its own signing step is expected to fail here (see above).
build_bundle() {
    (cd frontend && pnpm run tauri:build) || [[ -d "$BUNDLE/Contents/MacOS" ]]
}

# Copies the bundle without extended attributes and signs every executable.
sign_copy() {
    rm -rf "$OUT/Deino.app"
    mkdir -p "$OUT"
    ditto --noextattr --norsrc --noqtn "$BUNDLE" "$OUT/Deino.app"
    local helper
    for helper in "$OUT"/Deino.app/Contents/MacOS/*; do
        [[ "$(basename "$helper")" == deino ]] && continue
        codesign --force --options runtime --sign - "$helper" 2>/dev/null
    done
    codesign --force --options runtime --entitlements "$ENTITLEMENTS" --sign - "$OUT/Deino.app" 2>/dev/null
    codesign --verify --deep --strict "$OUT/Deino.app"
}

# Replaces the installed app; Deino must not be running.
install_app() {
    if pgrep -f "/Applications/Deino.app/Contents/MacOS/deino" >/dev/null; then
        echo "Quit Deino first (⌘Q), then run: scripts/build-app.sh --install" >&2
        return 1
    fi
    rm -rf /Applications/Deino.app
    ditto "$OUT/Deino.app" /Applications/Deino.app
    echo "Installed /Applications/Deino.app"
}

main() {
    stage_sidecars
    build_bundle
    sign_copy
    echo "Built and signed $OUT/Deino.app"
    [[ "${1:-}" == --install ]] && install_app
    return 0
}

main "$@"; exit
