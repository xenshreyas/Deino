# FIX-llama-openssl · llama-helper builds without OpenSSL

## Progress log (PARITY_PLAN §7.1)
`cargo build --release -p llama-helper --features metal` (`scripts/build-app.sh`
`stage_sidecars`, so `scripts/ship.sh`) and `scripts/clippy.sh` failed in a
cold target on Macs with Homebrew's OpenSSL 3: llama-cpp-sys-2 0.1.146 builds
llama.cpp's `common` library, whose vendored cpp-httplib then finds OpenSSL
(`LLAMA_OPENSSL` defaults to `ON`) and fails to compile against its newer
headers (`httplib.cpp`: `const X509_NAME *` into `X509_NAME *`). The root
`.cargo/config.toml` now sets `CMAKE_DISABLE_FIND_PACKAGE_OpenSSL = "ON"` in
`[env]`; llama-cpp-sys-2 forwards every `CMAKE_*` variable to CMake (it is the
only dependency that does), so llama.cpp builds cpp-httplib without HTTPS. The
helper only loads a local model and never makes a request, and the sidecar no
longer risks linking a Homebrew `libssl` that users' Macs don't have.
llama-cpp-2 and llama-cpp-sys-2 stay pinned at `=0.1.146`.

## CLAUDE.md
- Build prerequisites: OpenSSL is not needed (or used) to build the
  `llama-helper` sidecar; `.cargo/config.toml` keeps llama.cpp from looking
  for it.
