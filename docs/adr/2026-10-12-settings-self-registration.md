# Register settings sections, settings pages and ID files from their own files

- **Status:** Accepted
- **Date:** 2026-10-12
- **Card:** REG-02 (infrastructure; extends
  [feature self-registration](2026-10-12-feature-self-registration.md))

## Context

REG-01 removed the central lists of plugins, ACL, bindings, composition,
tables and stages. Parallel branches still collided on four more:

- `deino-core/src/settings/model.rs`: one `Settings` field per section, the
  `use super::{..}` list of section types, the `Default` impl and the
  `validate` chain; `settings.rs` added a `pub use <section>::{..}` per
  section; `patch.rs` listed the dialog-only paths (`DIALOG_ONLY`).
- the frontend settings registries: `sections.tsx` (one page entry and
  import each), `search.ts` (the `z.enum` of page ids) and `copy.ts` (each
  page's label and description).
- `deino-domain/src/id.rs`: a `mod` plus `pub use` line per ID file.
- the settings fixtures (`settings-fixture.ts`, `dev/mock-ipc/settings-store.ts`),
  which repeated every section's defaults by hand.

Constraints: no behaviour change (`settings.json` keeps its shape and key
order, existing files load, the generated `Settings` type stays the same,
pages keep their order and groups) and strict typing (§3A: no
`serde_json::Value` or `HashMap` sections, no `string` page ids).

## Decision

1. **Settings sections: generated from `register_section!` by `build.rs`.**
   Section files moved to `settings/sections/<file>.rs`. Each registers its
   sections in its own file:

   ```rust
   crate::settings::register_section! {
       /// Exports and auto-export (EXP-02).
       export: ExportSettings,
       order: 140,
       validate: ExportSettings::validate,
       dialog_only: ["folder"],
   }
   ```

   The macro implements `SettingsSection` (`check()` calls `validate`, or
   accepts everything). `deino-core/build.rs` (`build/settings.rs`) reads the
   invocations line by line and writes `$OUT_DIR/settings_model.rs`, one
   `settings! { sections: [..] dialog_only: [..] }` invocation sorted by
   `(order, key)`; the `settings!` macro in `model.rs` declares the same
   `Settings` struct as before (field docs from the `///` lines, serde and
   specta derives), its `Default`, `validate` (every section's `check`, in
   field order) and `DIALOG_ONLY` (from each `dialog_only`, as
   `(section, field, "section.field")`). It also writes
   `$OUT_DIR/settings_sections.rs`, `pub use <module>::*;` for every file in
   `sections/`, included by `settings/sections.rs`; `settings.rs` re-exports
   `sections::*`. A section file without its `mod` line in `sections.rs`
   fails the build with that instruction. Existing sections took orders 10
   to 240 in their old field order, so the struct, the JSON key order and
   the TypeScript type are unchanged.
2. **Settings pages: `import.meta.glob` over `features/*/settings-page.{ts,tsx}`.**
   Each feature exports `settingsPages` (`id`, `group`, `order`, `icon`,
   `label`, `description`, `Component`, checked with
   `satisfies readonly SettingsPage[]`) and adds its id to the
   `SettingsPageIds` interface of `@/features/settings/search` by
   declaration merging, so `SettingsSectionId` stays a literal union and
   `<Link search={{ section: 'calendar' }}>` is still type-checked.
   `sections.ts` loads the modules eagerly, checks them with zod (a broken
   page module throws), and sorts by group (`SETTINGS_GROUPS`), `order`,
   then id. The list is built on first use, not at import, because page
   modules import features that import the settings feature. The URL schema
   is `z.custom<SettingsSectionId>(isSettingsSectionId)`, so unknown values
   still fall back to the first page. The settings feature's own and
   combined pages (General, Appearance, Recording, Meeting detection,
   Transcription, Summary models, Integrations, Network, Beta, About) are in
   `features/settings/settings-page.tsx`; Calendar (with briefs) is in
   `features/calendar/`.
3. **Fixtures: from the generated defaults.** The bindings test also writes
   `generated/settings-defaults.ts`, `DEFAULT_SETTINGS: Settings` from
   `Settings::default()`. `settings-fixture.ts` clones it; the mock store
   spreads it and overrides only the returning user's values.
4. **IDs: `deino-domain/build.rs` re-exports every `src/id/*.rs`.** `id.rs`
   keeps one alphabetical `mod <area>;` per file and includes the generated
   `pub use <area>::*;` lines; a file without its `mod` line fails the
   build.
5. **The guard.** `scripts/check-feature-isolation.sh` fails when
   `model.rs` lists a section again, `patch.rs` declares `DIALOG_ONLY`,
   `settings.rs` names a section module, `sections.rs` holds anything but
   `mod` lines and the include, `id.rs` re-exports a file by hand,
   `sections.ts` imports a feature or lists an id, `search.ts` uses
   `z.enum`, `copy.ts` has page copy, or a fixture stops starting from
   `DEFAULT_SETTINGS`. `.gitattributes` adds `settings/sections.rs` and
   `id.rs` to the union merge.

## Alternatives

- **A `serde_json::Value` or `HashMap<String, ..>` of sections:** no list
  at all, but loses typing, the specta type and compile-time checks (§3A).
- **`linkme` for sections:** a slice can supply defaults and checks at
  runtime, but not the fields of a struct that serde and specta see.
- **`syn` in the build script:** sturdier parsing, but a new build
  dependency for a fixed, documented layout; the scanner is tested
  (`settings/section_scan_tests.rs`) and fails with a message.
- **Generated `#[path]` module declarations** (no `mod` line at all):
  rustfmt never reaches files declared in `$OUT_DIR`, and rust-analyzer
  handles them poorly (the REG-01 reasoning).
- **A Vite plugin writing a `settings-pages.gen.ts`:** static types without
  declaration merging, but one more generated file that every page branch
  changes and the merger must regenerate.
- **Alphabetical page order:** no `order` field, but changes the nav.

## Consequences

- A new section: its file in `settings/sections/` with `register_section!`
  plus `mod <file>;` in `sections.rs`. A new page: `settings-page.tsx` in
  its feature folder. New IDs: `id/<area>.rs` plus `mod <area>;` in `id.rs`.
  Fixtures follow on `cargo test -p deino --lib bindings`.
- `Settings::validate` checks sections in field order; the first error of a
  document that breaks several rules can differ. Nothing depends on it.
- Settings section types are re-exported by globs: a `pub` item in a
  section file is now public from `deino_core::settings`.
- The generated TypeScript gains `settings-defaults.ts`; `Settings` itself
  is byte-identical.
- Not covered: `SettingsPage` copy lives in each page file rather than a
  feature's `copy.ts`; the shell nav (`features/shell/nav.ts`), meeting
  slots and palette providers are still central registries.
