# deino-secrets

## Role

API keys and tokens, kept in the OS keychain (ADR 0013). Nothing else in
Deino stores a secret.

## Owns

- `SecretName`: the closed set of secrets, each its own keychain account
  (`openai_api_key`, `anthropic_api_key`, …, and ORG-01's
  `google_oauth_client_secret`, `google_calendar_refresh_token`,
  `microsoft_calendar_refresh_token`).
- `SecretValue`: a validated value that cannot be printed, serialized or
  cloned, and is wiped from memory on drop.
- `SecretStore`: `set`, `get`, `delete`, and `status` (`missing`, `stored`,
  `locked`), with its contract in `contract_tests.rs`.
- `KeychainSecretStore`: the macOS Keychain (native stores on Windows and
  Linux) through `keyring`, service = the app's bundle identifier.
- `CredentialKey` and `CredentialStore` (INT-00, ADR 0031): credentials whose
  keychain account is decided at run time (`integration.<provider>.<id>`),
  up to `MAX_CREDENTIAL_CHARS`; `KeychainSecretStore` implements both ports,
  `InMemoryCredentialStore` (feature `test-support`) is the fake. Denied
  access is `CredentialDenied` ("connect the integration again").
- `DatabaseKeyStore` (PRV-02, ADR 0041): the database encryption key as
  its 64-hex-digit text, one item under `DATABASE_KEY_ACCOUNT`
  (`database.encryption-key`); denied access is `DatabaseKeyDenied`.
  `InMemoryDatabaseKeyStore` (feature `test-support`) is the fake. The key's
  shape (`DbKey`) belongs to `deino-db`.
- `InMemorySecretStore` (feature `test-support`), which can simulate a
  keychain that refuses access (`deny`).

## Does Not Own

- IPC: the app's `features/secrets.rs` (`set`, `status`, `delete`; never a read).
- Which provider uses which secret: `deino_core::settings::LlmProvider::api_key`.
- Moving legacy plaintext keys: `deino_core::settings::LegacySqliteImport`.

## Invariants

- A secret never reaches a log, an error message, an IPC response or a file.
  Errors name the secret (`OpenAI API key`), never its value.
- The UI cannot read a secret back; `get` is for Rust consumers only.
- An item the OS refuses to hand over is `AccessDenied` / `locked`, not an
  internal error, so the UI can ask for the key again.
- Keychain calls block (sometimes on a system prompt), so they run on
  Tokio's blocking pool.
