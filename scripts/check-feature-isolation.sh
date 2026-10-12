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
    hits="$(grep -nvE '^\s*$|^\s*//|^#\[cfg\(test\)\]$|^(pub(\(crate\))? )?mod [a-z_]+;$|^include!\(concat!\(env!\("OUT_DIR"\), "/[a-z_]+\.rs"\)\);$' "$file" || true)"
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
# Module declarations don't count: every feature adds one, and that is allowed.
lines="$(grep -cvE '^(pub(\(crate\))? )?mod [a-z_0-9]+;$' "$APP/src/composition.rs")"
((lines <= 230)) || fail "$APP/src/composition.rs has $lines lines besides mod lines (cap 230): move feature wiring into an attach hook"

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

# REG-02: settings sections, settings pages and ID files register from their
# own files (docs/adr/2026-10-12-settings-self-registration.md).
readonly SETTINGS=crates/deino-core/src/settings
forbid "$SETTINGS/model.rs" '^\s+pub [a-z_]+: [A-Z][A-Za-z]*Settings,$|[A-Z][A-Za-z]*Settings::default\(\)|self\.[a-z_]+\.validate\(\)' \
    "lists settings sections; register_section! in the section's own file generates them"
forbid "$SETTINGS/patch.rs" 'const DIALOG_ONLY' "lists dialog-only settings; declare dialog_only in register_section!"
only_mod_lines "$SETTINGS/sections.rs"
for section in "$SETTINGS"/sections/*.rs; do
    module="$(basename "$section" .rs)"
    forbid "$SETTINGS.rs" "^(pub )?(mod|use) $module(;|::)" "names the section module $module; sections.rs declares it and build.rs exports it"
done
forbid crates/deino-domain/src/id.rs '^pub use [a-z_]+::' "re-exports an ID file; build.rs re-exports every file in id/"
readonly SETTINGS_UI=frontend/src/features/settings
forbid "$SETTINGS_UI/sections.ts" "from '@/features/|id: '" "lists settings pages; a feature registers its page in its own settings-page.tsx"
forbid "$SETTINGS_UI/search.ts" 'z\.enum\(' "lists settings page ids; each settings-page.tsx declares its id in SettingsPageIds"
forbid "$SETTINGS_UI/copy.ts" '^  sections: \{' "lists settings page copy; each page carries its label and description"
for fixture in "$SETTINGS_UI/settings-fixture.ts" frontend/src/dev/mock-ipc/settings-store.ts; do
    forbid "$fixture" '^\s+version: [0-9]+,' "copies the default settings; start from the generated DEFAULT_SETTINGS"
    grep -q 'DEFAULT_SETTINGS' "$fixture" || fail "$fixture does not start from the generated DEFAULT_SETTINGS"
done

if ((failures > 0)); then
    echo "check-feature-isolation: $failures problem(s); see docs/adr/2026-10-12-feature-self-registration.md" >&2
    exit 1
fi
echo "check-feature-isolation: no shared per-feature lists"
