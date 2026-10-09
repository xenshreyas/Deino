-- FND-06: settings moved to the typed settings file (settings.json) and API
-- keys moved to the macOS Keychain.
--
-- Before this migration runs, the Rust data migration
-- `deino_core::settings::LegacySqliteImport` (a `deino_db::DataMigration`
-- whose `runs_before` is this file's version) copies the model and
-- transcription settings into settings.json and every API key into the
-- Keychain. Dropping both tables then removes the plaintext keys from the
-- database. `transcript_settings` also held unused cloud-transcription keys
-- (Deino always transcribes locally); they go with it.
--
-- A plain DROP leaves the freed pages' bytes in the file until SQLite reuses
-- them. `secure_delete` makes SQLite overwrite them with zeros, so no key
-- survives on disk once the WAL is checkpointed (it is, on close). It is a
-- per-connection flag and is switched back off afterwards.

PRAGMA secure_delete = ON;

DROP TABLE IF EXISTS settings;
DROP TABLE IF EXISTS transcript_settings;

PRAGMA secure_delete = OFF;
