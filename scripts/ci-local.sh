#!/usr/bin/env bash
# The whole quality gate in one command (PARITY_PLAN §3A, FND-03). Run it
# before every merge. GitHub CI calls the same steps by name, so each check
# has exactly one definition: the step functions below.
#
#   scripts/ci-local.sh                  # every gate step, cheapest first
#   scripts/ci-local.sh rust frontend    # groups (see --list)
#   scripts/ci-local.sh clippy test      # single steps
#   scripts/ci-local.sh --list           # every step and group
#
# It stops at the first failing step and prints a timing summary. Compiles go
# through sccache (scripts/dev-env.sh). Needs: rustup, pnpm, cargo-deny,
# sqlx-cli 0.8 and typos-cli; cargo-nextest is installed when missing.

# The step_* functions are called by name from main.
# shellcheck disable=SC2329
set -euo pipefail

cd "$(dirname "$0")/.."
# shellcheck source=scripts/dev-env.sh
source scripts/dev-env.sh

# Every workspace member except the Tauri app and the llama.cpp sidecar: the
# crates that build anywhere without native SDKs (Linux CI lints and tests them).
PORTABLE=(--workspace --exclude deino --exclude llama-helper)
readonly GENERATED_BINDINGS="frontend/src/lib/ipc/generated"

FRONTEND_STEPS=(pnpm-install typecheck lint format vitest vite-build e2e)
RUST_STEPS=(fmt file-size deps deny clippy test bindings sqlx)
# Cheap, compile-free checks first, so a typo fails in seconds, not minutes.
ALL_STEPS=(fmt file-size deps deny "${FRONTEND_STEPS[@]}" clippy test bindings sqlx)
# The steps that load the whole machine (or flake when it is loaded). Only
# these wait for the machine-wide gate lock.
HEAVY_STEPS=(e2e clippy test bindings sqlx test-app clippy-portable test-portable)
# Not in the default run: Linux CI's stand-ins for clippy and test.
EXTRA_STEPS=(check clippy-portable test-portable test-app fmt-fix test-touched)
# The feature agent's gate (CLAUDE.md "Merge workflow"): formats first, then the
# checks that catch most problems, testing only the crates the branch touched.
# The full gate runs once per merge batch, on the batch branch.
FAST_STEPS=(fmt-fix file-size clippy typecheck vitest test-touched)

# ── Steps ────────────────────────────────────────────────────────────────────

step_fmt() { cargo fmt --all --check; }
step_file_size() { scripts/check-rust-file-size.sh; }
# Dependency rules, and REG-01's rule that no central file lists features.
step_deps() {
    scripts/check-dependency-rules.sh
    scripts/check-feature-isolation.sh
}

step_deny() {
    cargo deny --locked check licenses bans sources
    # Advisories warn instead of failing: the database changes daily, so a new
    # entry must not turn unrelated work red. Triage each one in deny.toml.
    if ! cargo deny --locked check advisories; then
        warn "cargo deny found new advisories; triage them in deny.toml"
    fi
}

step_pnpm_install() { pnpm --dir frontend install --frozen-lockfile; }
step_typecheck() { pnpm --dir frontend typecheck; }
step_lint() { pnpm --dir frontend lint; }
step_format() { pnpm --dir frontend format:check; }
step_vitest() { pnpm --dir frontend test; }
# `pnpm build` is typecheck + vite build; the typecheck step already ran.
step_vite_build() { pnpm --dir frontend exec vite build; }
# Playwright smoke tests in the installed Chrome over `pnpm dev:mock`;
# screenshots land in frontend/test-results/.
# One retry: under heavy load a click can time out before Vite has compiled.
step_e2e() { pnpm --dir frontend test:e2e --retries=1; }

step_clippy() {
    stage_sidecar
    scripts/clippy.sh
}

step_clippy_portable() { scripts/clippy.sh "${PORTABLE[@]}"; }

step_test() {
    stage_sidecar
    run_tests --workspace
}

step_test_portable() { run_tests "${PORTABLE[@]}"; }

# Formats the workspace instead of checking it, so the fast gate never fails on fmt.
step_fmt_fix() { cargo fmt --all; }

# Tests only the crates this branch changed since it left origin/main, and the
# app crate when the frontend's Rust side changed. Skips when no crate changed.
step_test_touched() {
    local crates=() packages=() crate
    read -r -a crates <<<"$(touched_crates)"
    if ((${#crates[@]} == 0)); then
        echo "test-touched: no Rust crate changed; skipping"
        return 0
    fi
    stage_sidecar
    echo "test-touched: ${crates[*]}"
    for crate in "${crates[@]}"; do packages+=(-p "$crate"); done
    cargo test --locked --no-fail-fast "${packages[@]}" --lib --tests --bins
}

# The packages whose directories hold a file changed since the merge base with
# origin/main (committed or not), as one space-separated line.
touched_crates() {
    local base
    base="$(git merge-base HEAD origin/main 2>/dev/null || git rev-parse HEAD)"
    { git diff --name-only "$base"; git diff --name-only; git ls-files --others --exclude-standard; } |
        sort -u | python3 -I -c '
import json, subprocess, sys
meta = json.loads(subprocess.run(["cargo", "metadata", "--no-deps", "--format-version", "1"],
                                 capture_output=True, text=True, check=True).stdout)
root = meta["workspace_root"] + "/"
dirs = sorted(((p["manifest_path"][len(root):].rsplit("/", 1)[0] + "/", p["name"])
               for p in meta["packages"]), key=lambda d: -len(d[0]))
hit = set()
for path in sys.stdin.read().split():
    for prefix, name in dirs:
        if path.startswith(prefix):
            hit.add(name)
            break
print(" ".join(sorted(hit)))'
}

# Linux CI: the app's unit tests, from the test binary `bindings` builds anyway.
step_test_app() {
    stage_sidecar
    run_unit_tests -p deino --lib
}

# Linux CI: the whole workspace compiles with the macOS-only code cfg'd out.
step_check() {
    stage_sidecar
    cargo check --workspace --all-targets --locked
}

step_bindings() {
    stage_sidecar
    cargo test --locked -p deino --lib bindings
    local stale
    stale="$(git status --porcelain -- "$GENERATED_BINDINGS")"
    if [[ -n "$stale" ]]; then
        git --no-pager diff -- "$GENERATED_BINDINGS"
        echo "error: $GENERATED_BINDINGS is stale:" >&2
        echo "$stale" >&2
        echo "Commit the regenerated files (cargo test -p deino --lib bindings)." >&2
        return 1
    fi
}

# Last among the Rust steps: it touches deino-core's sources to force the
# query macros to re-run, which invalidates later incremental builds.
step_sqlx() {
    stage_sidecar
    scripts/sqlx-prepare.sh --check
}

# ── Helpers ──────────────────────────────────────────────────────────────────

warn() {
    if [[ -n "${GITHUB_ACTIONS:-}" ]]; then
        echo "::warning::$*"
    else
        echo "warning: $*" >&2
    fi
}

# Unit and integration tests, then the doctests, which nextest cannot run.
run_tests() {
    run_unit_tests "$@"
    cargo test --locked --doc "$@"
}

# Through nextest (one process per test) when it is available.
# Crates whose test binaries link the macOS media and ML frameworks. Starting
# such a binary costs 1-5 s, so nextest's process per test makes each of their
# tests that slow; libtest runs them as threads in one process, about 7x faster.
# Every other crate stays on nextest, which is faster for many small tests.
HEAVY_TEST_CRATES=(deino deino-audio deino-pipeline deino-diarize deino-detect deino-recording deino-speaker-tags)

run_unit_tests() {
    if ! ensure_nextest; then
        cargo test --locked "$@"
    elif [[ "$*" == "--workspace" ]]; then
        run_split_tests
    else
        cargo nextest run --locked "$@"
    fi
}

# The whole workspace: the rest through nextest, then the heavy crates
# in-process. One after the other, so the two never compete for the target
# directory or the CPU.
run_split_tests() {
    local heavy=() exclude=() crate
    for crate in "${HEAVY_TEST_CRATES[@]}"; do
        heavy+=(-p "$crate")
        exclude+=(--exclude "$crate")
    done
    cargo nextest run --locked --workspace "${exclude[@]}"
    cargo test --locked --no-fail-fast "${heavy[@]}" --lib --tests --bins
}

ensure_nextest() {
    command -v cargo-nextest >/dev/null 2>&1 && return 0
    echo "ci-local: installing cargo-nextest" >&2
    # A Homebrew bottle installs in seconds; cargo install compiles for minutes.
    if command -v brew >/dev/null 2>&1 && brew install cargo-nextest; then
        return 0
    fi
    if cargo install cargo-nextest --locked; then
        return 0
    fi
    warn "could not install cargo-nextest; falling back to cargo test"
    return 1
}

# tauri-build refuses to compile the app unless every `externalBin` exists.
# Checks and tests never run the llama-helper or `deino` CLI sidecars, so when
# the real ones are not staged (fresh clones, worktrees, CI), stage stubs that
# explain themselves (scripts/stage-cli.sh stages the real CLI).
stage_sidecar() {
    stage_stub llama-helper "Build the real sidecar as the root README describes, then replace this file."
    stage_stub cli "Run scripts/stage-cli.sh to build and stage the real deino CLI."
}

# Stages a placeholder for sidecar $1 unless one is there; $2 says how to get the real one.
stage_stub() {
    local triple sidecar
    triple="$(rustc -vV | sed -n 's/^host: //p')"
    sidecar="frontend/src-tauri/binaries/$1-$triple"
    [[ -e "$sidecar" ]] && return 0
    mkdir -p "$(dirname "$sidecar")"
    printf '#!/bin/sh\necho "%s: this is a placeholder staged by scripts/ci-local.sh." >&2\necho "%s" >&2\nexit 1\n' \
        "$1" "$2" >"$sidecar"
    chmod +x "$sidecar"
    warn "staged a placeholder $1 sidecar at $sidecar (enough to build and test)"
}

# Fails before any step runs if a tool the selected steps need is missing.
require_tools() {
    local missing=() tool
    for tool in "$@"; do
        command -v "$tool" >/dev/null 2>&1 || missing+=("$tool")
    done
    ((${#missing[@]} == 0)) && return 0
    echo "error: missing tools: ${missing[*]}" >&2
    echo "  cargo-deny: cargo install cargo-deny --locked" >&2
    echo "  sqlx:       cargo install sqlx-cli --version 0.8.6 --locked --no-default-features --features sqlite" >&2
    echo "  typos:      cargo install typos-cli --locked" >&2
    echo "  pnpm:       brew install pnpm" >&2
    exit 2
}

tools_for() {
    case "$1" in
        pnpm-install | typecheck | lint | format | vitest | vite-build | e2e) echo pnpm ;;
        deny) echo cargo cargo-deny ;;
        sqlx) echo cargo sqlx ;;
        clippy | clippy-portable) echo cargo typos ;;
        *) echo cargo ;;
    esac
}

# ── Driver ───────────────────────────────────────────────────────────────────

usage() {
    sed -n '2,13p' "$0" | sed 's/^# \{0,1\}//'
    echo
    echo "Steps:  ${ALL_STEPS[*]}"
    echo "Extra:  ${EXTRA_STEPS[*]}"
    echo "Groups: all, rust (${RUST_STEPS[*]}), frontend (${FRONTEND_STEPS[*]}),"
    echo "        fast (${FAST_STEPS[*]})"
}

# Expands groups and validates step names; prints one step per line.
resolve() {
    local arg
    for arg in "$@"; do
        case "$arg" in
            all) printf '%s\n' "${ALL_STEPS[@]}" ;;
            rust) printf '%s\n' "${RUST_STEPS[@]}" ;;
            frontend) printf '%s\n' "${FRONTEND_STEPS[@]}" ;;
            fast) printf '%s\n' "${FAST_STEPS[@]}" ;;
            *)
                if ! declare -F "step_${arg//-/_}" >/dev/null; then
                    echo "error: unknown step '$arg' (see --list)" >&2
                    return 2
                fi
                echo "$arg"
                ;;
        esac
    done
}

summary() {
    local status="$1" line
    echo
    echo "── ci-local summary ─────────────────────────────"
    for line in "${results[@]}"; do
        echo "  $line"
    done
    printf '  %-16s %5ss\n' total "$((SECONDS - started))"
    echo "  load average: $(uptime | sed 's/.*load averages*: //')"
    echo "─────────────────────────────────────────────────"
    echo "$status"
}

# ── Machine-wide gate slots ──────────────────────────────────────────────────
# Parallel worktrees each running the full gate overloaded the machine (load
# 100+), and timing-sensitive tests then flaked. So the heavy steps run in one
# of DEINO_GATE_SLOTS slots (default 2); other gates wait for a free one. A
# slot whose owner died is reclaimed. DEINO_GATE_NO_LOCK=1 skips this.
# Slot 0 keeps the original lock path, so older copies of this script share it.
readonly GATE_LOCK_DIR="${HOME}/.cache/deino"
GATE_SLOT=""

# The lock directory of slot $1.
slot_path() { (($1 == 0)) && echo "$GATE_LOCK_DIR/gate.lock" || echo "$GATE_LOCK_DIR/gate.lock.$1"; }

# Claims slot $1 if it is free or its owner died; fails otherwise.
try_slot() {
    local path owner
    path="$(slot_path "$1")"
    mkdir "$path" 2>/dev/null && return 0
    owner="$(cat "$path/pid" 2>/dev/null || true)"
    [[ -n "$owner" ]] && ! kill -0 "$owner" 2>/dev/null && rm -rf "$path" && mkdir "$path" 2>/dev/null
}

acquire_gate_lock() {
    [[ "${DEINO_GATE_NO_LOCK:-}" == 1 || -n "$GATE_SLOT" ]] && return 0
    mkdir -p "$GATE_LOCK_DIR"
    local slots="${DEINO_GATE_SLOTS:-2}" waited=0 slot
    while true; do
        for ((slot = 0; slot < slots; slot++)); do
            try_slot "$slot" && GATE_SLOT="$slot" && break 2
        done
        ((waited % 60 == 0)) && echo "Waiting for a free gate slot ($slots in use)…"
        sleep 5
        waited=$((waited + 5))
    done
    echo $$ >"$(slot_path "$GATE_SLOT")/pid"
    trap release_gate_lock EXIT
    # Gates holding a slot are what the queue waits on, so they get more
    # compile jobs than an ordinary build.
    export CARGO_BUILD_JOBS="${DEINO_GATE_JOBS:-4}"
}

release_gate_lock() {
    [[ -z "$GATE_SLOT" ]] && return 0
    local path
    path="$(slot_path "$GATE_SLOT")"
    [[ "$(cat "$path/pid" 2>/dev/null)" == "$$" ]] && rm -rf "$path"
    return 0
}

main() {
    case "${1:-}" in
        -h | --help | --list) usage && return 0 ;;
    esac
    local steps step began status needed tools=()
    steps="$(resolve "${@:-all}")"
    for step in $steps; do
        read -r -a needed <<<"$(tools_for "$step")"
        tools+=("${needed[@]}")
    done
    require_tools "${tools[@]}"

    results=()
    started=$SECONDS
    for step in $steps; do
        echo
        echo "━━ $step ━━"
        began=$SECONDS
        # Light steps run unlocked; the lock is taken before the first heavy one.
        [[ " ${HEAVY_STEPS[*]} " == *" $step "* ]] && acquire_gate_lock
        # A subshell outside any `if`, so `set -e` stops a step at its first
        # failing command (bash ignores -e inside a function called from `if`).
        set +e
        (
            set -e
            "step_${step//-/_}"
        )
        status=$?
        set -e
        if ((status != 0)); then
            results+=("$(printf '%-16s %5ss  FAILED' "$step" "$((SECONDS - began))")")
            summary "FAILED at '$step'. Fix it and re-run: scripts/ci-local.sh $step"
            return 1
        fi
        results+=("$(printf '%-16s %5ss  ok' "$step" "$((SECONDS - began))")")
    done
    summary "PASSED"
}

# `exit` on the same line: bash never reads past it, so editing this file
# while a run is in progress cannot corrupt that run.
main "$@"; exit
