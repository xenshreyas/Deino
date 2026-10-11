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
# Not in the default run: Linux CI's stand-ins for clippy and test.
EXTRA_STEPS=(check clippy-portable test-portable test-app)

# ── Steps ────────────────────────────────────────────────────────────────────

step_fmt() { cargo fmt --all --check; }
step_file_size() { scripts/check-rust-file-size.sh; }
step_deps() { scripts/check-dependency-rules.sh; }

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
step_e2e() { pnpm --dir frontend test:e2e; }

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
run_unit_tests() {
    if ensure_nextest; then
        cargo nextest run --locked "$@"
    else
        cargo test --locked "$@"
    fi
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
    echo "Groups: all, rust (${RUST_STEPS[*]}), frontend (${FRONTEND_STEPS[*]})"
}

# Expands groups and validates step names; prints one step per line.
resolve() {
    local arg
    for arg in "$@"; do
        case "$arg" in
            all) printf '%s\n' "${ALL_STEPS[@]}" ;;
            rust) printf '%s\n' "${RUST_STEPS[@]}" ;;
            frontend) printf '%s\n' "${FRONTEND_STEPS[@]}" ;;
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

# ── Machine-wide lock ────────────────────────────────────────────────────────
# Parallel worktrees each running the full gate overloads the machine (load
# 100+), and timing-sensitive tests then flake. One gate runs at a time; the
# others wait. A lock whose owner died is reclaimed. DEINO_GATE_NO_LOCK=1 skips it.
readonly GATE_LOCK="${HOME}/.cache/deino/gate.lock"

acquire_gate_lock() {
    [[ "${DEINO_GATE_NO_LOCK:-}" == 1 ]] && return 0
    mkdir -p "$(dirname "$GATE_LOCK")"
    local waited=0 owner
    until mkdir "$GATE_LOCK" 2>/dev/null; do
        owner="$(cat "$GATE_LOCK/pid" 2>/dev/null || true)"
        if [[ -n "$owner" ]] && ! kill -0 "$owner" 2>/dev/null; then
            rm -rf "$GATE_LOCK"
            continue
        fi
        ((waited % 60 == 0)) && echo "Waiting for another gate run (pid ${owner:-?}) to finish…"
        sleep 5
        waited=$((waited + 5))
    done
    echo $$ >"$GATE_LOCK/pid"
    trap release_gate_lock EXIT
}

release_gate_lock() {
    [[ "$(cat "$GATE_LOCK/pid" 2>/dev/null)" == "$$" ]] && rm -rf "$GATE_LOCK"
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
    acquire_gate_lock

    results=()
    started=$SECONDS
    for step in $steps; do
        echo
        echo "━━ $step ━━"
        began=$SECONDS
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
