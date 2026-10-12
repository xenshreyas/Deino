# 0041. Encrypt the database with SQLCipher and a random Keychain key

- **Status:** Accepted (backend; the app wiring and app lock follow, see `docs/handoff/PRV-02.md`)
- **Date:** 2026-10-11
- **Card:** PRV-02 (encryption at rest and app lock)

## Context

Granola-class apps encrypt their local store. Deino keeps transcripts,
notes and people in `deino.sqlite` (WAL, one writer, a read pool; ADR
0012), read by the app and, from another process, by the read-only `deino`
CLI (ADR 0035). The card asks for optional encryption, plaintext by
default, one binary that opens both, a key that never reaches disk or logs,
and a lossless, crash-safe migration both ways. Audio stays plaintext.

## Decision

1. **SQLCipher through the libsqlite3-sys sqlx already links.** `deino-db`
   enables `libsqlite3-sys = "=0.30.1"` with `bundled-sqlcipher` on Unix
   (the version sqlx 0.8.6 pins, so features unify into one SQLite). It
   compiles SQLCipher 4.5.7 Community (SQLite 3.45.3) from the crate's
   amalgamation. Without `OPENSSL_*` variables, Apple targets build with
   `SQLCIPHER_CRYPTO_CC` and link Security.framework (CommonCrypto): **no
   OpenSSL** is vendored or linked on macOS. Linux links the system
   `libcrypto` (CI already installs `libssl-dev`). Windows keeps plain
   SQLite; `cipher_available()` reports it and keyed opens fail with
   `CipherUnavailable` instead of silently writing plaintext.
2. **One binary, both formats.** Without a key SQLCipher is SQLite.
   `FileFormat::detect` reads the 16-byte header (`SQLite format 3\0` or
   empty = plaintext, anything else = encrypted). `Database::open_keyed`
   and `open_read_only_keyed` take `Option<&DbKey>`; the old constructors
   pass `None`. A file and a key that do not fit fail early and clearly:
   `KeyRequired`, `NotEncrypted`, `WrongKey`. None of these count as
   corruption, so the app never deletes WAL side files over a database it
   simply cannot read.
3. **The key.** `DbKey`: 32 random bytes from `getrandom`, held in
   `Zeroizing`, no `Debug`/`Display`/`Clone`, constant-time `Eq`. It is
   stored as 64 hex digits in one Keychain item (`DatabaseKeyStore`,
   account `database.encryption-key`, service = bundle id) and given to
   SQLCipher as a raw key (`x'…'`), so opening skips PBKDF2. Keyed
   connections run `PRAGMA key` first (sqlx orders it before every other
   pragma) and **disable sqlx statement logging**, because the pragma text
   carries the key and sqlx logs slow statements at `warn` by default.
4. **Conversion (`deino_db::convert`).** On a closed database: open the
   original, fingerprint it, `ATTACH … AS converted KEY ?` (bound, not in
   the SQL text) to `<db>.converting`, `sqlcipher_export`, copy
   `user_version`, set WAL, detach and close. Refuse to continue if a
   `-wal` file survives the close (another process has it open). Verify
   the copy (`quick_check` plus `Fingerprint`: every table's row count and
   SHA-256 over its sorted `quote()`d rows, shadow tables included, plus
   the schema). Then rename original → `<db>.backup`, copy → `<db>`, fsync
   the folder, verify again, delete the backup (or rename it back).
   `recover()` resolves a crash: backup without database → restore it;
   backup beside database → the swap finished, drop the backup; a lone
   copy → delete it.

## Alternatives

- **`bundled-sqlcipher-vendored-openssl`:** builds OpenSSL from source
  (minutes, a second crypto stack, more advisories). CommonCrypto is the
  platform's FIPS-validated library and costs nothing to build.
- **`rusqlite` or a second SQLite for the cipher:** two `links = "sqlite3"`
  crates cannot coexist; one SQLite for everything is simpler.
- **A passphrase (PBKDF2):** slower opens on every pool connection and a
  secret the user can forget; the Keychain already guards API keys.
- **Encrypting audio:** out of scope; recordings can live on an encrypted
  APFS volume (`docs/setup/encrypted-recordings.md`).

## Consequences

- Build cost: `libsqlite3-sys` compiles the SQLCipher amalgamation instead
  of SQLite's, about the same size: 16 s cold for the crate at load ~56 on
  the M4 Max (sccache then caches it). No new crate is downloaded;
  `cargo deny check licenses bans sources` passes. SQLCipher's BSD notice
  is in `THIRD_PARTY_NOTICES.md`. SQLite moves from 3.46.0 to 3.45.3 (every
  migration and test passes).
- The key also lives, unzeroized, in sqlx's connect options for as long as
  the pools live (SQLCipher itself keeps it in memory too). It never reaches
  disk or logs; a debugger on the running app could read it.
- The plaintext backup is deleted with `remove_file`. On an SSD that is not
  a secure erase; FileVault is the answer to raw-disk recovery.
- The `deino` CLI reads the Keychain only when the file is encrypted; macOS
  may prompt once per binary signature ("Always Allow").
