#!/usr/bin/env bash
# FND-12 / PARITY_PLAN §4.0: deino-net owns the only HTTP client, and audio
# and transcription have no route to the network at all.
#
#   scripts/check-net-isolation.sh
#
# 1. deino-audio and deino-stt must not depend, even indirectly, on deino-net
#    or any HTTP client (the `cargo tree -i reqwest` check, made robust to
#    two reqwest versions in the tree). A crate that does not exist yet is
#    skipped, so this runs before CAP-01 lands.
# 2. No crate under crates/ except deino-net depends directly on an HTTP
#    client. (The app crate's inherited LLM clients still do until FND-09 and
#    CAP-01 move them onto deino-net; clippy's `disallowed-types` covers the
#    app's new code.)
set -euo pipefail

cd "$(dirname "$0")/.."

readonly ISOLATED=(deino-audio deino-stt)
readonly HTTP_CLIENTS='^(reqwest|hyper|ureq|isahc|surf|attohttpc|curl) '

failures=0

# The packages `crate` depends on through normal and build edges, one per line.
dependencies() {
    cargo tree --locked --package "$1" --edges normal,build --prefix none "${@:2}"
}

for crate in "${ISOLATED[@]}"; do
    if ! cargo pkgid --locked "$crate" >/dev/null 2>&1; then
        echo "check-net-isolation: $crate does not exist yet; skipped"
        continue
    fi
    if dependencies "$crate" | grep -Eq "^deino-net |$HTTP_CLIENTS"; then
        echo "error: $crate reaches the network: it must not depend on deino-net or an HTTP client" >&2
        dependencies "$crate" --invert reqwest 2>/dev/null >&2 || true
        failures=$((failures + 1))
    fi
done

for manifest in crates/*/Cargo.toml; do
    crate="$(sed -n 's/^name = "\(.*\)"$/\1/p' "$manifest" | head -n 1)"
    [[ "$crate" == "deino-net" ]] && continue
    if dependencies "$crate" --depth 1 | tail -n +2 | grep -Eq "$HTTP_CLIENTS"; then
        echo "error: $crate depends on an HTTP client; send requests through deino-net" >&2
        failures=$((failures + 1))
    fi
done

if ((failures > 0)); then
    exit 1
fi
echo "Network isolation OK (deino-net is the only HTTP client; audio and STT stay offline)"
