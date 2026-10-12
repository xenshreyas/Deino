#!/usr/bin/env bash
# REG-01: adding a feature edits no shared list. Each feature registers its
# plugin, commands, bindings, wiring, tables and pipeline stages from its own
# files (docs/adr/2026-10-12-feature-self-registration.md). This fails when a
# central file grows a per-feature list again; the only shared lines allowed
# are one alphabetical `mod` declaration per module.
#
# It also proves the rule on the reference feature: `_example` may appear in
# the central files only as its `mod` line.
#
#   scripts/check-feature-isolation.sh
set -euo pipefail

cd "$(dirname "$0")/.."

readonly APP=frontend/src-tauri
failures=0

fail() {
    echo "error: $*" >&2
    failures=$((failures + 1))
}

# Fails when `file` has a line matching the extended regex `pattern`.
forbid() {
    local file="$1" pattern="$2" why="$3" hits
    hits="$(grep -nE "$pattern" "$file" || true)"
    if [[ -n "$hits" ]]; then
        fail "$file: $why"
        echo "$hits" >&2
    fi
}

# Fails when `file` has any line that is not blank, a comment, an attribute or
# a `mod` declaration (the module files that may only declare modules).
only_mod_lines() {
    local file="$1" hits
    hits="$(grep -nvE '^\s*$|^\s*//|^#\[cfg\(test\)\]$|^(pub(\(crate\))? )?mod [a-z_]+;$' "$file" || true)"
    if [[ -n "$hits" ]]; then
        fail "$file may only declare modules; register a feature from its own file"
        echo "$hits" >&2
    fi
}

# The retired central lists stay gone.
for retired in "$APP"/src/features/{manifest,organize_manifest,recording_manifest,manifest_commands,dev_manifest}.rs \
    "$APP"/src/composition/background.rs "$APP"/capabilities/dev; do
    [[ -e "$retired" ]] && fail "$retired is back; features register themselves (REG-01)"
done

# features.rs: module declarations only (a debug-only feature says so in its own file).
only_mod_lines "$APP/src/features.rs"

# lib.rs: plugins come from features::registry, never a list of init() calls.
forbid "$APP/src/lib.rs" 'features::[a-z_]+::init\(' "registers a feature plugin by name; use register_plugin! in its file"

# The capabilities grant no feature to the main window: build.rs generates that grant.
plugin_names="$(grep -hoE '^pub const NAME: &str = "[a-z-]+";' "$APP"/src/features/*.rs | cut -d'"' -f2 | paste -sd'|' -)"
forbid "$APP/capabilities/default.json" "\"($plugin_names):default\"" "grants a feature plugin; build.rs generates the main window's grants"

# The lock policy lives in each plugin's registration, not a list of names.
forbid "$APP/src/features/privacy/guard.rs" 'OPEN_WHILE_LOCKED' "lists plugins open while locked; declare register_plugin!(Open) instead"

# composition.rs: leaf wiring runs from its attach hook, never from here too.
while read -r hooked; do
    module="$(basename "$hooked" .rs)"
    while read -r wiring; do
        forbid "$APP/src/composition.rs" "$module::$wiring\(" "calls $module::$wiring, which runs from its attach hook"
    done < <(grep -A6 -E 'attach::attach!\(' "$hooked" | grep -oE 'attach_[a-z_]+\(' | tr -d '(' | sort -u)
done < <(grep -lE 'attach::attach!\(' "$APP"/src/composition/*.rs)
lines="$(wc -l <"$APP/src/composition.rs")"
((lines <= 230)) || fail "$APP/src/composition.rs has $lines lines (cap 230): move feature wiring into an attach hook"

# bindings.rs lists no builder.
forbid "$APP/src/bindings.rs" 'features::[a-z_]+::specta' "lists a bindings builder; plugins register their own"

# The reference feature appears in central code only as its `mod` line.
for central in "$APP/src/lib.rs" "$APP/src/features.rs" "$APP/src/composition.rs" "$APP/src/setup.rs" \
    "$APP/src/bindings.rs" "$APP/build.rs" "$APP"/capabilities/*.json; do
    hits="$(grep -nE '_example|"example' "$central" | grep -vE '^[0-9]+:\s*//|^[0-9]+:(pub )?mod _?example;$' || true)"
    [[ -z "$hits" ]] || { fail "$central names _example outside its mod line"; echo "$hits" >&2; }
done

# deino-db: tables come from the migrations; the frozen list never grows.
forbid crates/deino-db/src/table.rs '=> Self::|Self::[A-Z][A-Za-z]+,$' "lists tables; build.rs generates them from the migrations"
frozen="$(grep -cE '^\s+\("[a-z_]+",' crates/deino-db/build/legacy_tables.rs)"
((frozen <= 51)) || fail "crates/deino-db/build/legacy_tables.rs grew to $frozen tables; it is frozen at 51"

# deino-pipeline: stages register themselves with register_stage!.
forbid crates/deino-pipeline/src/registry.rs 'Stage::new\(ports\)' "lists a stage; use register_stage! in its file"

# deino-domain: IDs are re-exported by a glob.
forbid crates/deino-domain/src/deino_domain.rs 'pub use id::\{' "lists ID types; \`pub use id::*\` re-exports them"

if ((failures > 0)); then
    echo "check-feature-isolation: $failures problem(s); see docs/adr/2026-10-12-feature-self-registration.md" >&2
    exit 1
fi
echo "check-feature-isolation: no shared per-feature lists"
