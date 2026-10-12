# REG-01 · Features register themselves; no shared per-feature lists

## Progress log (PARITY_PLAN §7.1)
REG-01: every feature registers its plugin, commands, ACL, capability grant,
bindings, composition, tables and pipeline stages from its own files
([ADR 2026-10-12](../adr/2026-10-12-feature-self-registration.md)).
`build.rs` generates the ACL and the `main` window's grants from each
`collect_commands!`; `linkme` slices `features::registry::PLUGINS`
(`register_plugin!(Guarded | Open)`), `composition::attach::ATTACHMENTS`
(`attach!(name, order, Phase::…)`) and `deino_pipeline::registry::STAGES`
(`register_stage!`); `deino-db/build.rs` generates `Table` from the
migrations; `deino-domain` re-exports `id::*`. No behaviour change: the same
plugins, commands, permissions, lock policy, stage order and tables; the
bindings differ only in `Table`'s doc sentence. `scripts/check-feature-isolation.sh`
(gate step `deps`) keeps the central files list-free.

## Follow-ups (PARITY_PLAN §7.2)
- Infra: `StagePorts` and `composition/pipeline.rs`'s `StageAdapters` still
  take one field per new stage port; a stage needing a new port edits them.
- Infra: `setup.rs`'s platform steps (`attach_detection`, `attach_developer`,
  `attach_permissions`, `attach_stt`) could become `Phase::Startup` hooks if
  their order relative to the tray and engines is pinned first.
- Infra: `deino-pipeline/src/stages.rs` re-exports each stage's types.

## Architecture (ARCHITECTURE.md)
- Extension points: replace "add a line to the manifest, `lib.rs`,
  `capabilities/default.json` and `features::builders()`" with: a feature
  plugin calls `super::registry::register_plugin!(Guarded)` (or `Open` to keep
  working while locked) in `features/<f>.rs`; `build.rs` reads its
  `collect_commands!` for the ACL and the capability; leaf wiring calls
  `super::attach::attach!("<f>", <order>, Phase::Local | ReadModel |
  Background | Startup(..))` in `composition/<f>.rs`.
- New pipeline stage: `crate::registry::register_stage!(MyStage);` in
  `stages/my_stage.rs` (plus its `mod` line); its `order()` places it.
- Database: `Table` is generated from the migrations (`deino-db/build.rs`);
  a new table's doc is the `--` comment block directly above its
  `CREATE TABLE`. `build/legacy_tables.rs` is frozen.
- The app crate: `features/registry.rs` (the plugin slice, `Access`),
  `features/acl.rs` (the generated ACL and its guard tests),
  `composition/attach.rs` (`Phase`, `Context`, the hook slice),
  `composition/settings.rs`, `composition/services.rs`,
  `composition/example.rs`; `composition/background.rs` and the
  `features/*manifest*.rs` files are gone, as is `capabilities/dev/`.

- The `features.rs`, `features/` rows (both copies): drop "`features/manifest.rs`
  lists plugins and commands for the ACL; debug-only plugins are in
  `DEV_FEATURES` (`features/dev_manifest.rs`) and granted by `capabilities/dev/`"
  for "each registers itself (`register_plugin!`); `build.rs` generates the
  ACL and grants from its `collect_commands!`; a debug-only plugin declares
  `#![cfg(debug_assertions)]`". In the calendar row, "command list in
  `organize_manifest.rs`" goes.
- "Extension points" table: "A feature" → copy `_example`, then one `pub mod`
  line in `features.rs` and one `mod` line in `composition.rs`; nothing else
  shared. "A table" → "a migration with a `--` doc comment above its
  `CREATE TABLE`, then `scripts/sqlx-prepare.sh` and the bindings test".

## CLAUDE.md
- "Merge workflow", "Registration is per feature": point at the ADR and
  `scripts/check-feature-isolation.sh`.
- The app crate: `features.rs`, `features/`: "one inline plugin per feature,
  each registering itself (`register_plugin!`); `build.rs` generates their ACL
  and capability from each `collect_commands!`". `composition.rs`: "the shared
  services; leaf features wire themselves with attach hooks
  (`composition/attach.rs`)".
- `crates/deino-pipeline` row: "`registry.rs` (stages register themselves
  with `register_stage!`)" instead of "one line per stage".
- Replace "How to add a feature: copy `_example`" steps 1 and 2 with:

  1. **Core:** `crates/deino-core/src/_example.rs` + `_example/` →
     `crates/deino-core/src/<feature>.rs` + `<feature>/{model,error,repo,events,service,fake}.rs`
     (plus `pub mod <feature>;` in `deino_core.rs`). Put the migration in
     `crates/deino-db/migrations/<UTC timestamp>_<card>_<desc>.sql` with a
     `--` comment directly above each `CREATE TABLE` (it becomes the `Table`
     variant's doc; `build.rs` generates the variant), then run
     `scripts/sqlx-prepare.sh`. New ID types go in
     `crates/deino-domain/src/id.rs` (re-exported by a glob).
  2. **Plugin:** `frontend/src-tauri/src/features/_example.rs` + `_example/` →
     `features/<feature>.rs` + `<feature>/{commands,events,contract_tests}.rs`.
     In `features/<feature>.rs`, below `NAME`, write
     `super::registry::register_plugin!(Guarded);` (`Open` only for a plugin
     that must work while Deino is locked, a privacy decision that updates
     `privacy::guard`'s test). Add `pub mod <feature>;` to `features.rs` in
     alphabetical order; that is the only shared line. `build.rs` reads
     `commands.rs`'s `collect_commands![..]` and generates the plugin's ACL
     and the `main` window's grant; there is no manifest or capability to
     edit. A debug-only feature puts `#![cfg(debug_assertions)]` in its own
     file. Wire services in `composition/<feature>.rs` (plus `mod <feature>;`
     in `composition.rs`) and register them with
     `super::attach::attach!("<feature>", <order>, Phase::Local(|cx| ..))`
     (`ReadModel` for wiring over the LLM gateway, `Background` for services
     that run on their own, `Startup` for a feature with its own store). Then
     `cargo test -p deino --lib bindings`.
  3. **UI:** unchanged.
- Rules that are easy to break: "Never add a per-feature line to a central
  file (`lib.rs`, `features.rs` beyond its `pub mod`, `composition.rs`,
  `capabilities/default.json`, `bindings.rs`, `table.rs`, `registry.rs`);
  `scripts/check-feature-isolation.sh` fails on it."
- Dependencies: `linkme` (=0.3.37) is the one registration mechanism; its
  macros' `#[allow(unsafe_code)]` lives only in the three registration macros.

## Plan updates
- §3 rule 2 (Migrations): "Add a `Table` variant" → "document each new table
  with a `--` comment above its `CREATE TABLE`; `Table` is generated".
- §3 rule 3 (Where code goes, IPC): "Register it with one line each in
  `features/manifest.rs`, `lib.rs` and `capabilities/default.json`" →
  "Register it from its own file with `register_plugin!`; `build.rs`
  generates its ACL and capability".
- §4.2 / §3A Open-closed: pipeline stages register with `register_stage!`.
- ADR 0002 and ADR 0007: add "Amended by ADR 2026-10-12 (feature
  self-registration)" to their status lines.
