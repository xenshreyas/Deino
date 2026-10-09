# 0011. Enforce the code-quality limits with tools, and exempt legacy code in one list

- **Status:** Accepted
- **Date:** 2026-10-08
- **Card:** FND-13 (PARITY_PLAN §3A, `docs/references/architecture.md` §8, §11)

## Context

PARITY_PLAN §3A sets hard limits (40-line functions, 300-line files, 4
parameters, nesting depth 3, cognitive complexity 15), zero-tolerance typing
and architecture rules (no direct HTTP client, no raw `tokio::spawn`, time
from `Clock`). Limits that live only in prose drift. The inherited code
breaks most of them, so turning them on everywhere would either fail every
build or require a rewrite.

## Decision

**Rust**
- `[workspace.lints]` in the root `Cargo.toml` (every crate opts in): clippy
  `pedantic`, `too_many_lines`, `cognitive_complexity`, `too_many_arguments`,
  `excessive_nesting`, no `unwrap`/`expect`/`panic`/`dbg!`/`todo!`/printing,
  `allow_attributes_without_reason`, `let_underscore_must_use`, `missing_docs`,
  `unsafe_code` and `unused_must_use`.
- `clippy.toml` holds the thresholds (40/15/4, nesting 5 = depth 3 inside a
  method) and `disallowed-types`/`disallowed-methods` with reasons:
  `reqwest::Client`, `tokio::spawn`, `SystemTime::now`, `Instant::now`,
  `Utc::now`, `thread::sleep`. The one sanctioned reader of the wall clock is
  `SystemClock`.
- `scripts/clippy.sh` (`-D warnings`, plus typos), `rustfmt.toml`,
  `rust-toolchain.toml` (1.96.1), `deny.toml` (permissive licences only),
  `scripts/check-rust-file-size.sh`.

**TypeScript**
- ESLint `strictTypeChecked`, react-hooks, TanStack Query, sonarjs, the size
  limits and import boundaries; Prettier with the Tailwind plugin; tsconfig
  `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`,
  `noImplicitOverride`.

**Legacy, in one list per language, that only shrinks**
- Rust: `frontend/src-tauri/src/legacy_lints.rs` declares inherited modules
  through `legacy_modules!` with the exact lints they still break;
  `scripts/legacy-rust-files.txt` lists files over 300 lines.
- Frontend: `frontend/legacy-files.mjs` downgrades the strict ESLint rules to
  warnings, keeps strict-flag type errors from blocking (a baseline
  `tsconfig.legacy.json` still applies), and skips Prettier for those files.
- A card that ports a file removes it from the list in the same PR; the size
  check fails on stale entries.

## Alternatives

- **Warnings only:** ignored within weeks.
- **Rewrite legacy code first:** months of work with no feature value; cards
  rewrite what they touch instead.
- **Per-file suppressions:** scattered and easy to add silently;
  `allow_attributes_without_reason` makes every new one explain itself.

## Consequences

- New code cannot merge while breaking a limit; legacy debt stays visible.
- `rustfmt` cannot see modules declared inside `legacy_modules!`, so legacy
  files keep their inherited formatting until ported (ported files are
  formatted then).
- Async Tauri commands need one documented `let_underscore_must_use`
  exemption per `commands.rs`, because the command macro expands to a
  `let _` type check.
