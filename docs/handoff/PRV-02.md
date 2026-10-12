# PRV-02 handoff: second half (app wiring, app lock, Settings)

Backend and tests are done (ADR 0038, crate READMEs). Read them and the card.

## Done (do not redo)

- `deino-db`: SQLCipher linked (Unix; CommonCrypto on macOS). `DbKey`,
  `FileFormat::detect`, `Database::open_keyed(location, Option<&DbKey>,
  migrator, steps)`, `open_read_only_keyed`, `cipher_available()`,
  `convert(path, Conversion::Encrypt|Decrypt(&key))`, `recover(path)`,
  `Fingerprint`. Errors: `KeyRequired`, `WrongKey`, `NotEncrypted`,
  `AlreadyEncrypted`, `CipherUnavailable`, `Mismatch`, `InUse`, `Swap`.
- `deino-secrets`: `DatabaseKeyStore` (`save/load/forget_database_key`,
  account `database.encryption-key`) on `KeychainSecretStore`;
  `InMemoryDatabaseKeyStore` fake. Store the key as `DbKey::to_hex()`.
- `deino-cli`: `db_key::key_for` reads the key only for an encrypted file.
- Docs: ADR 0038, `docs/setup/encrypted-recordings.md`, ARCHITECTURE, §7.

## Left

1. **Setting.** `PrivacySettings` (`crates/deino-core/src/settings/personal.rs`)
   gets `encrypt_database: bool` (default false) and `app_lock: AppLock
   { enabled: bool, idle_minutes: u16 }` (default off, 5 min). Merge
   patches already flow; then `cargo test -p deino --lib bindings`.
2. **Startup conversion** (`frontend/src-tauri/src/database/manager.rs`,
   `new_from_app_handle`, before `Self::open`): read the setting;
   `deino_db::recover(&db_path)`; if the file is plaintext and the setting is
   on → `DbKey::generate()`, `save_database_key` **first**, then
   `convert(Encrypt)`; if encrypted and off → load key, `convert(Decrypt)`,
   then `forget_database_key`. Open with `open_keyed(.., key.as_ref(), ..)`.
   Put the decision in a small `deino-core::privacy` service over
   `DatabaseKeyStore` + a `DatabaseFile` port so it is unit-testable; keep
   the manager thin. `WrongKey`/`KeyRequired` at startup → a calm blocking
   error window, never a delete. Changing the toggle = "Restart to apply".
3. **App lock** (new module, e.g. `frontend/src-tauri/src/features/lock.rs`
   + `deino-core::lock`): a `LockState` (`Unlocked`/`Locked`) service driven
   by a `Clock`, locked after `idle_minutes` without UI activity and on
   wake/screen lock (`NSWorkspaceDidWakeNotification`,
   `com.apple.screenIsLocked`). Unlock with LocalAuthentication
   `LAPolicyDeviceOwnerAuthentication` (Touch ID or password) via `objc2`
   behind `#[cfg(target_os = "macos")]`, stub elsewhere. Commands: `status`,
   `unlock`, `activity` (UI pings, throttled), `lock_now`.
4. **IPC guard:** while locked, every data-reading plugin command returns
   `AppError` `permission_denied`. Cheapest seam: a check in each feature
   plugin's `on_invoke` or a shared guard in `error.rs`/`features.rs`;
   exempt `lock:*`, `system`, `recording` (recording keeps running and its
   status/stop stay usable). Test the guard with a contract test.
5. **UI:** `features/lock/` (lock screen over the shell: app name, a lock
   icon, "Unlock with Touch ID" button, recording pill if recording) and a
   Settings → Privacy & security section via `features/settings/sections.tsx`
   (group `privacy`): encryption toggle with restart notice, app lock toggle,
   idle minutes select, a link to the APFS doc. Tokens only; light and dark
   screenshots via `pnpm dev:mock` (add fixtures to the mock IPC).
6. Tick §7.1, close the §7.2 row, ARCHITECTURE (lock plugin), then the gate.

Gotchas:
- Never call `forget_database_key` before the file is plaintext again.
- `convert` needs the database closed: run it before any pool opens
  (setup.rs opens the DB early; legacy `commands.rs` also opens one).
- Tests never use service `com.xenshreyas.deino`; Keychain tests `#[ignore]`.
