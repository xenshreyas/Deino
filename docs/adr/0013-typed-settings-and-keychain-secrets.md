# 0013. Keep settings in one typed file and API keys in the Keychain

- **Status:** Accepted
- **Date:** 2026-10-09
- **Card:** FND-06 (`docs/references/architecture.md` (b) FND-06)

## Context

Before FND-06, preferences lived in five places: two single-row SQLite tables
(`settings`, with every LLM API key in plaintext, and `transcript_settings`),
four tauri-plugin-store JSON files, the notification manager's own JSON file
and the webview's localStorage. Each had its own reader, so the same value
(the summary model, say) was loaded three times and kept in sync with an
untyped `model-config-updated` event. The UI could read every key back
(`api_get_api_key`). PARITY_PLAN §3 rule 5 requires a typed settings store
and Keychain secrets for anything new.

## Decision

- **One document.** `deino_core::settings::Settings` is a versioned struct of
  sections (`ai`, `audio`, `recording`, `transcription`, `notifications`,
  `privacy`, `profile`, `ui`, `beta`, `onboarding`), each `#[serde(default)]`
  so a file from any older build loads. It is stored as
  `<app_config_dir>/settings.json` by `JsonFileSettingsStore`, which writes a
  temp file, `fsync`s it, renames it and `fsync`s the directory. An unreadable
  file is set aside and defaults are used, never a failed launch.
- **One owner.** `SettingsService` keeps the current value in memory,
  serialises changes, validates before saving, and announces each saved
  change through the `SettingsEvents` port (`SettingsChanged`, carrying the
  whole document, so the UI cache is written, not refetched).
- **Merge patches at the edge.** The UI sends RFC 7396 merge patches
  (`plugin:settings|patch`). The core merges the patch into the current
  value, parses the result back into `Settings` and rejects unknown fields,
  wrong types and broken rules as `invalid` before anything is saved. Rust
  callers use a typed closure (`SettingsService::update`).
- **Secrets are not settings.** `deino-secrets` owns the `SecretStore` port:
  `set`, `status` (`missing`, `stored`, `locked`), `delete`, and `get` for
  Rust consumers only. The `secrets` plugin exposes no read command and
  `SecretValue` cannot be serialized or printed. `KeychainSecretStore` uses
  the `keyring` crate with the bundle identifier as the service, so a build
  with another identifier never sees the real keys.
- **Lost Keychain access is a state, not a crash.** An existing item that
  macOS refuses to hand over (`errSecAuthFailed`, a denied prompt, no UI) is
  `AccessDenied`/`locked`; the key field tells the user to enter the key
  again, and a summary that needs it fails with that message.
- **Legacy stores are imported once and then removed.** The SQLite rows move
  through `LegacySqliteImport`, a new `deino_db::DataMigration` that runs
  against the schema just before the migration that drops both tables; that
  migration uses `secure_delete`, so no key bytes survive in the file. The
  JSON stores move at startup (`LegacyFileImport`) and are deleted; the UI
  moves its localStorage keys (`SettingsSync`). Each importer is idempotent
  and skips, with a warning, any value today's rules reject.

## Alternatives

- **A `settings` key-value table in SQLite.** Typed access would need a
  schema per key, and the Rust side would still parse JSON per value. A file
  keeps the document readable, needs no migration per new setting, and
  cannot lock the database.
- **tauri-plugin-store for the document.** Untyped (`serde_json::Value` per
  key), saved on its own schedule, and reachable from the UI around the
  validation. Removed instead.
- **Reading keys in the UI and passing them to commands.** That is how the
  legacy code worked; it puts every key in the webview's memory and in
  any log that prints arguments.
- **Running the SQLite import before opening the database, with dynamic
  column reads.** It would have to cope with every historical schema; the
  `DataMigration` step always sees the exact schema just before the drop.

## Consequences

- New settings are a field with a default; no migration. A change that
  defaults cannot express bumps `SETTINGS_VERSION` and adds an upgrade.
- Settings are not in SQLite, so `DbChanged` does not cover them; the UI
  listens to `SettingsChanged` instead (`useSettingsChanges`).
- Ad-hoc-signed builds may prompt for Keychain access after each rebuild;
  that is macOS behaviour, surfaced as `locked` when denied.
- The `DataMigration` hook is available to any later card that has to move
  data out of the database before a schema change.
