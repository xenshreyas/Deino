#!/usr/bin/env bash
# PARITY_PLAN §3A: no Rust source file over 300 lines, tests excluded.
#
# A file's inline test module (`#[cfg(test)]` followed by `mod name {`, which
# clippy keeps at the end of the file) does not count, and neither do sibling
# `*_tests.rs` files or anything under `tests/`. Inherited files over the cap
# are listed in legacy-rust-files.txt next to this script; that list only shrinks.
#
#   scripts/check-rust-file-size.sh
set -euo pipefail

cd "$(dirname "$0")/.."

readonly MAX_LINES=300
readonly LEGACY_LIST="scripts/legacy-rust-files.txt"

# Lines of a file before its inline test module.
counted_lines() {
    awk '
        /^#\[cfg\(test\)\]$/ { pending = 1; next }
        pending && /^mod [A-Za-z0-9_]+ \{/ { exit }
        { count += 1 + pending; pending = 0 }
        END { print count + 0 }
    ' "$1"
}

is_legacy() {
    grep -qxF "$1" <(grep -v '^#' "$LEGACY_LIST")
}

failures=0
while IFS= read -r file; do
    lines="$(counted_lines "$file")"
    if ((lines > MAX_LINES)) && ! is_legacy "$file"; then
        echo "error: $file has $lines lines (max $MAX_LINES, tests excluded); split it by responsibility" >&2
        failures=$((failures + 1))
    elif ((lines <= MAX_LINES)) && is_legacy "$file"; then
        echo "error: $file is down to $lines lines; remove it from $LEGACY_LIST" >&2
        failures=$((failures + 1))
    fi
done < <(git ls-files --cached --others --exclude-standard -- '*.rs' |
    grep -vE '(_tests\.rs$|(^|/)tests/)')

while IFS= read -r file; do
    if [[ ! -f "$file" ]]; then
        echo "error: $file no longer exists; remove it from $LEGACY_LIST" >&2
        failures=$((failures + 1))
    fi
done < <(grep -v '^#' "$LEGACY_LIST")

if ((failures > 0)); then
    exit 1
fi
echo "Rust file sizes OK (max $MAX_LINES lines, tests excluded)"
