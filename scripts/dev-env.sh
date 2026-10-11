#!/usr/bin/env bash
# Source this before building Deino (`source scripts/dev-env.sh`).
#
# It turns on the sccache compile cache for Rust and for the C/C++ parts
# (whisper.cpp, llama.cpp, onnxruntime glue), so parallel agent worktrees
# share compiled artifacts instead of each rebuilding about 12 GB from scratch.
# If sccache is missing it does nothing, so the build still works.

if command -v sccache >/dev/null 2>&1; then
  export RUSTC_WRAPPER="sccache"
  export CMAKE_C_COMPILER_LAUNCHER="sccache"
  export CMAKE_CXX_COMPILER_LAUNCHER="sccache"
  export SCCACHE_CACHE_SIZE="${SCCACHE_CACHE_SIZE:-80G}"
  # Dependencies are cached by sccache. Workspace crates keep incremental
  # compilation, which sccache skips, so edit-compile loops stay fast.
  # Several agents build in parallel on one machine. Capping jobs per build
  # avoids oversubscription (load 140+ on 16 cores), which made every build slower.
  export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-6}"
fi

# A new agent worktree starts with no `target/`, so its first full build
# recompiles ~400 crates plus the C++ engines (45 min at high load). Seed it
# with a copy-on-write clone of the main checkout's `target/` (APFS clonefile:
# about 30 s and no extra disk until files diverge). Afterwards only the
# workspace crates rebuild. DEINO_NO_SEED=1 skips this.
deino_seed_target() {
  local root main
  root="$(git rev-parse --show-toplevel 2>/dev/null)" || return 0
  main="$(git worktree list --porcelain 2>/dev/null | sed -n '1s/^worktree //p')"
  [[ -n "$main" && "$root" != "$main" && -d "$main/target" ]] || return 0
  # A second shell racing the first sees target.seeding and builds cold.
  [[ -e "$root/target" || -e "$root/target.seeding" || "$(uname)" != Darwin ]] && return 0
  echo "dev-env: seeding target/ from $main (copy-on-write)…" >&2
  cp -c -R "$main/target" "$root/target.seeding" && mv "$root/target.seeding" "$root/target"
}
[[ "${DEINO_NO_SEED:-}" == 1 ]] || deino_seed_target
