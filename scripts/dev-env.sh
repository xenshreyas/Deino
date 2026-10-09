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
