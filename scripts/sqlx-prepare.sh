#!/usr/bin/env bash
# Regenerates the committed `.sqlx/` data that `sqlx::query*!` macros compile
# against (SQLX_OFFLINE=true in .cargo/config.toml). Run it after adding a
# migration or changing a query, and commit the result.
#
#   scripts/sqlx-prepare.sh            # regenerate
#   scripts/sqlx-prepare.sh --check    # CI: fail if .sqlx/ is stale
#
# Needs sqlx-cli 0.8 (matching the sqlx crate):
#   cargo install sqlx-cli --version 0.8.6 --locked --no-default-features --features sqlite
set -euo pipefail

cd "$(dirname "$0")/.."

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

# A throwaway database with every schema the macros check against.
export DATABASE_URL="sqlite://$scratch/schema.sqlite"
export SQLX_OFFLINE=false
sqlx database create
sqlx migrate run --source crates/deino-db/migrations
# The debug-only `_example` feature keeps its own schema (see its module docs).
sqlx migrate run --source crates/deino-core/src/_example/migrations --ignore-missing

cargo sqlx prepare --workspace "$@" -- --all-targets
