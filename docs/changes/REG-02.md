# REG-02 · Settings and IDs register per feature

## Progress log (PARITY_PLAN §7.1)
REG-02 (infra, [ADR](../adr/2026-10-12-settings-self-registration.md)):
settings sections, settings pages, settings fixtures and ID files register
from their own files. `register_section!` + `deino-core/build.rs` generate
`Settings` (struct, `Default`, `validate`, the patch's `DIALOG_ONLY`) from
`settings/sections/*.rs`; `features/*/settings-page.{ts,tsx}` export
`settingsPages`, collected by `import.meta.glob` in
`features/settings/sections.ts`, with ids added to `SettingsPageIds` by
declaration merging; the bindings test writes
`generated/settings-defaults.ts` (`DEFAULT_SETTINGS`), which both settings
fixtures start from; `deino-domain/build.rs` re-exports every
`src/id/*.rs`. No behaviour change: today's full `settings.json`
(`crates/deino-core/tests/fixtures/settings-full.json`) round-trips
byte-for-byte, the generated `Settings` type is unchanged, the nav order is
unchanged. `check-feature-isolation.sh` guards all of it.

## Follow-ups (PARITY_PLAN §7.2)
- REG-03: the shell nav (`features/shell/nav.ts`), meeting slots and
  palette providers are still central lists; the same glob pattern applies.
- Owner QA: page labels and descriptions now live in each
  `settings-page.tsx` rather than a feature's `copy.ts`.

## Architecture (ARCHITECTURE.md)
- Settings (deino-core): sections live in `settings/sections/<file>.rs`,
  each registering itself with `crate::settings::register_section! { /// doc
  key: Type, order: N, [validate: Type::validate,] [dialog_only: ["field"],] }`.
  `build.rs` (`build/settings.rs`) generates the `Settings` struct (fields by
  `order`, then key), `Default`, `validate` and `DIALOG_ONLY`, and the glob
  re-exports of every section file. `SettingsSection::check` is a section's
  rule check.
- Frontend settings: `features/settings/sections.ts` collects
  `features/*/settings-page.{ts,tsx}` (`settingsPages: SettingsPage[]`:
  `id`, `group`, `order`, `icon`, `label`, `description`, `Component`),
  sorted by `SETTINGS_GROUPS`, `order`, id. `SettingsSectionId` is
  `keyof SettingsPageIds` (`features/settings/search.ts`), extended by each
  page file. `generated/settings-defaults.ts` holds `DEFAULT_SETTINGS`
  (stays real in `dev:mock`, like `shared.ts`).
- deino-domain: `build.rs` re-exports every `src/id/*.rs`.

## CLAUDE.md
- "How to add a feature", step 1: "New ID types go in
  `crates/deino-domain/src/id/<area>.rs` plus one alphabetical `mod <area>;`
  in `id.rs` (`build.rs` re-exports the file)."
- Patterns, Settings: "A new setting is a field with a default in its
  section. A new section is a file in `crates/deino-core/src/settings/sections/`
  with `crate::settings::register_section! { .. }` (pick an unused `order`)
  plus one alphabetical `mod` line in `settings/sections.rs`; then
  `cargo test -p deino --lib bindings` (it also regenerates the fixtures'
  `DEFAULT_SETTINGS`)."
- Rules that are easy to break, the registry list: replace
  "`features/settings/sections.tsx`" with "a feature's own
  `settings-page.tsx` (settings pages: export `settingsPages` and add the id
  to `SettingsPageIds`)".
- Merge workflow, registration: add "settings sections, settings pages and
  ID files ([REG-02](docs/adr/2026-10-12-settings-self-registration.md))".

## Plan updates
- REG-02 done.
