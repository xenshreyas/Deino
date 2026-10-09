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
  # Incremental compilation defeats sccache; the workspace crates rebuild fast anyway.
  export CARGO_INCREMENTAL=0
fi
