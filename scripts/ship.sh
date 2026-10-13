#!/usr/bin/env bash
# The fast lane: from a small fix to the owner's installed app in minutes.
#
#   scripts/ship.sh            # quick checks, build, install, relaunch
#   scripts/ship.sh --no-check # skip the quick checks (assets only)
#
# Only the checks that cover what changed run here (seconds): type-check,
# lint and Vitest for frontend changes; clippy and the touched crates' tests
# when Rust changed. The full gate (scripts/ci-local.sh) runs afterwards, off
# the critical path, and anything it finds is fixed forward.
#
# Deino is quit and reopened only when no meeting is recording.
set -euo pipefail
cd "$(dirname "$0")/.."

readonly APP="/Applications/Deino.app"
readonly DB="$HOME/Library/Application Support/com.xenshreyas.deino/deino.sqlite"
readonly BUILT="$HOME/Library/Caches/deino-build/Deino.app"

# Files changed against origin/main, committed or not.
changed_files() {
    { git diff --name-only origin/main...HEAD; git diff --name-only HEAD; } | sort -u
}

# The quick checks for what changed; nothing to check is a pass.
quick_checks() {
    local files steps=(fmt-fix)
    files="$(changed_files)"
    grep -q '^frontend/src/' <<<"$files" && steps+=(typecheck lint vitest)
    grep -qE '\.rs$|Cargo\.toml$' <<<"$files" && steps+=(clippy test-touched)
    [[ ${#steps[@]} -eq 1 ]] && return 0
    scripts/ci-local.sh "${steps[@]}"
}

# True while a meeting is being recorded (never interrupt one).
recording() {
    [[ -f "$DB" ]] || return 1
    [[ "$(sqlite3 -readonly "$DB" "select count(*) from meetings where status = 'recording';")" != 0 ]]
}

# Quits Deino politely and waits for it to exit.
quit_app() {
    osascript -e 'tell application id "com.xenshreyas.deino" to quit' >/dev/null 2>&1 || true
    local tries=0
    while pgrep -f "$APP/Contents/MacOS/deino" >/dev/null && ((tries++ < 40)); do
        sleep 0.25
    done
    ! pgrep -f "$APP/Contents/MacOS/deino" >/dev/null
}

install_and_open() {
    if recording; then
        echo "A meeting is recording; built but not installed. Run again after it ends." >&2
        return 1
    fi
    quit_app || { echo "Deino did not quit; install skipped." >&2; return 1; }
    rm -rf "$APP"
    ditto "$BUILT" "$APP"
    open -a "$APP"
    echo "Installed and reopened $APP"
}

main() {
    local start=$SECONDS
    [[ "${1:-}" == --no-check ]] || quick_checks
    echo "checks: $((SECONDS - start))s"
    scripts/build-app.sh
    echo "build: $((SECONDS - start))s"
    install_and_open
    echo "total: $((SECONDS - start))s"
}

main "$@"
