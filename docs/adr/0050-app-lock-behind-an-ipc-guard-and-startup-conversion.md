# 0050. The app lock is an IPC guard; encryption converts at startup

- **Status:** Accepted
- **Date:** 2026-10-11
- **Card:** PRV-02 (encryption at rest and app lock); builds on ADR 0041

## Context

ADR 0041 gave `deino-db` SQLCipher, `convert` and `recover`, and a Keychain
key store. The app still had to use them, and the card also asks for an
optional Touch ID / password lock after N idle minutes. Three questions
were open: when the file is converted, what "locked" protects, and how a
lock reaches every command without touching each feature.

## Decision

1. **Convert at startup, before any connection opens.** `convert` needs a
   closed file, and the legacy code expects `AppState` from the first
   command, so the database is converted when the app starts, not when the
   switch is flipped (Settings says "Restart Deino to apply").
   `deino_core::privacy::StartupConversion` runs `recover`, then encrypts or
   decrypts to match `privacy.encryptDatabase`, over a `DatabaseFile` port
   and `DatabaseKeyStore`. The key is saved **before** encrypting and
   forgotten only **after** a verified decrypt. A failed conversion is not
   fatal: the file is unchanged, so it opens as it is and the outcome is
   shown in Settings. Only an encrypted file whose key cannot be read stops
   the open; the UI then explains the problem and nothing is deleted.
2. **Progress without blocking the window.** A normal start opens the
   database in the setup hook as before. When a conversion is pending, it
   runs on the async runtime and `LibraryStatus` (the `privacy` plugin's
   `library` command) reports `converting`; the root route's `LibraryGate`
   shows a calm progress screen until it is `ready`.
3. **The lock protects the window, not the file.** `LockService` is a
   `Clock`-driven state machine (`Unlocked`/`Locked`): it locks after the
   idle time (UI activity pings restart it), on wake and on screen lock,
   and unlocks through an `Authenticator` port (`LAContext` with
   `LAPolicyDeviceOwnerAuthentication`: Touch ID, falling back to the
   password). The database stays open and recording never stops.
4. **One guard for every command.** Every feature plugin is registered
   wrapped in `Guarded`, a `tauri::plugin::Plugin` that forwards everything
   to the real plugin and checks the lock in `extend_api`, the single path
   all plugin commands take. The legacy handler is wrapped the same way.
   While locked, library commands return `permission_denied`; `privacy`,
   `system`, `settings`, `recording` and `detection` stay open so the lock
   screen, the theme and an ongoing recording keep working. The UI unmounts
   the app while locked, so nothing refetches behind the lock screen.

## Alternatives

- **Convert when the switch is flipped:** every pool, the legacy
  `AppState` and the services hold connections; closing and reopening all
  of them live is a far bigger change than a restart.
- **A check inside each command, or in each plugin's `init`:** about 30
  edits that a new feature can forget. The wrapper is one line in `lib.rs`
  that covers new plugins automatically.
- **Locking by closing the database:** stops recording (it writes
  segments as they are spoken) and gains little: a running app already
  holds the key in memory (ADR 0041).
- **`objc2-local-authentication`:** a new crate for one message send;
  `LAContext` is reached through the objc2 runtime already in the tree.

## Consequences

- While a conversion runs, legacy state is not managed yet; the UI is
  gated, but Rust code that calls `app.state::<AppState>()` directly in
  that window would panic. Startup code uses `try_state`.
- The lock is only as strong as the window: a debugger or another process
  of the same user can still read the open database. FileVault and the
  macOS login protect the Mac; the lock protects an unattended window.
- Off macOS there is no prompt to unlock with, so the lock never engages
  there (`app_lock::supported`), whatever the setting says.
