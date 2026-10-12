#!/usr/bin/env bash
# Merges a batch of ready feature branches and runs the full gate once on the
# result (CLAUDE.md "Merge workflow"). Only the merger runs this.
#
#   scripts/merge-batch.sh feat/A feat/B feat/C   # merge, gate, report
#   scripts/merge-batch.sh --land                 # fast-forward main to the batch
#
# It works in its own worktree (.claude/worktrees/merge-batch) on the branch
# `merge-batch`, made from origin/main. Branches whose merge conflicts outside
# the generated files are skipped and listed, so their agent can rebase. On a
# conflict in generated bindings or `.sqlx/`, it keeps the batch's version and
# regenerates both after the last merge. rerere replays known resolutions.
# Written for macOS's bash 3.2: no mapfile, associative arrays or ${x,,}.
set -euo pipefail
cd "$(dirname "$0")/.."
readonly ROOT="$PWD"
readonly TREE="$ROOT/.claude/worktrees/merge-batch"
readonly GENERATED='^(frontend/src/lib/ipc/generated/|\.sqlx/|crates/[^/]+/\.sqlx/)'

# Fast-forwards origin/main and the main checkout to the gated batch.
land() {
    git -C "$TREE" push origin merge-batch:main
    git -C "$ROOT" merge --ff-only origin/main
    echo "landed: main is now $(git -C "$ROOT" rev-parse --short HEAD)"
}

# A fresh worktree on `merge-batch` at origin/main.
prepare_tree() {
    git fetch -q origin
    if [[ -d "$TREE" ]]; then
        git -C "$TREE" merge --abort 2>/dev/null || true
        git -C "$TREE" checkout -q -B merge-batch origin/main
    else
        git worktree add -q -B merge-batch "$TREE" origin/main
    fi
}

# Merges branch $1 into the batch; returns 1 (and undoes it) on a real conflict.
merge_one() {
    local branch="$1" conflicted output
    if output="$(git -C "$TREE" merge -q --no-ff --no-edit "origin/$branch" -m "Merge $branch" 2>&1)"; then
        return 0
    fi
    conflicted="$(git -C "$TREE" diff --name-only --diff-filter=U)"
    if [[ -z "$conflicted" ]]; then
        echo "skip $branch: merge failed without conflicts: $output"
        git -C "$TREE" merge --abort 2>/dev/null || true
        return 1
    fi
    if grep -qvE "$GENERATED" <<<"$conflicted"; then
        git -C "$TREE" merge --abort
        echo "skip $branch: conflicts in $(grep -vE "$GENERATED" <<<"$conflicted" | tr '\n' ' ')"
        return 1
    fi
    local file
    while IFS= read -r file; do
        git -C "$TREE" checkout -q --ours -- "$file"
        git -C "$TREE" add -- "$file"
    done <<<"$conflicted"
    git -C "$TREE" commit -q --no-edit
}

# Regenerates the TS bindings and the sqlx offline data, committing any change.
regenerate() {
    (cd "$TREE" && cargo test -q --locked -p deino --lib bindings >/dev/null)
    (cd "$TREE" && scripts/ci-local.sh sqlx >/dev/null 2>&1) || true
    if [[ -n "$(git -C "$TREE" status --porcelain)" ]]; then
        git -C "$TREE" add -A
        git -C "$TREE" commit -q -m "chore(batch): regenerate bindings and sqlx data"
    fi
}

main() {
    [[ "${1:-}" == "--land" ]] && { land; return; }
    (($# > 0)) || { sed -n '2,12p' "$0"; return 2; }
    prepare_tree
    local merged=() branch
    for branch in "$@"; do
        merge_one "$branch" && merged+=("$branch")
    done
    ((${#merged[@]} > 0)) || { echo "nothing merged"; return 1; }
    # shellcheck source=scripts/dev-env.sh
    (cd "$TREE" && source scripts/dev-env.sh && regenerate)
    echo "batch: ${merged[*]}"
    # The batch is the critical path: give its gate most of the machine.
    if (cd "$TREE" && DEINO_GATE_JOBS="${DEINO_GATE_JOBS:-12}" MOCK_PORT="${MOCK_PORT:-3400}" scripts/ci-local.sh); then
        echo "GATE PASSED: run scripts/merge-batch.sh --land"
    else
        echo "GATE FAILED: bisect by re-running with half the branches"
        return 1
    fi
}

main "$@"; exit
