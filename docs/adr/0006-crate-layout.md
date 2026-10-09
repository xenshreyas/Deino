# 0006. Split the Rust code into Tauri-free crates, features as modules

- **Status:** Accepted
- **Date:** 2026-10-08
- **Card:** FND-13 (PARITY_PLAN §4.0, `docs/references/architecture.md` §1)

## Context

The inherited core was one crate where commands, SQL, business rules and OS
calls mixed, so nothing could be tested without Tauri and every change
recompiled everything. anarlog splits into about 150 crates plus 45 plugin
crates; Handy and Cap keep one large crate. The plan needs the core to run
without a webview for the CLI, MCP and REST cards (DEV-02/03/04).

## Decision

Three tiers:

1. **Layer crates in `crates/`, none of which depends on `tauri`:**
   `deino-domain` (IDs, `AppError`, `Clock`), `deino-db` (pools, migrations,
   change signals), `deino-core` (features), `deino-testkit` (fixtures and
   fakes, dev-only). Later cards add `deino-pipeline`, `deino-llm`,
   `deino-net`, `deino-secrets`, `deino-audio`, `deino-stt` and `deino-doc`
   when they start. Dependencies point one way:
   domain ← db ← core ← pipeline ← app.
2. **Features are modules in `deino-core`:** `<feature>.rs` plus
   `<feature>/{model,error,repo,service,fake}.rs`. A feature becomes a crate
   only when it gains heavy native dependencies or passes about 3k lines.
3. **IPC lives in the app crate** as one inline plugin per feature
   (ADR 0002, ADR 0007), wired by the composition root `composition.rs`.

Each crate names its library root after itself (`src/deino_core.rs`), has a
`//!` doc and a README with Role, Owns, Does Not Own and Invariants. Fakes
live next to their trait behind a `test-support` feature.

## Alternatives

- **One crate per feature:** strong boundaries, but a dozen tiny crates now
  and a manifest per feature for little gain.
- **Plugin crates per feature (anarlog):** separate permissions, JS packages
  and manifests; overhead for a single app.
- **Keep one crate:** no enforced boundary between Tauri and logic.

## Consequences

- `cargo test -p deino-core` runs in seconds with no webview.
- The dependency rules can be checked mechanically (`cargo tree`, FND-03).
- `deino-testkit` must never depend on `deino-core`: core's tests use it, and
  the cycle would compile two copies of core's types.
