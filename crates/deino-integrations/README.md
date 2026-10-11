# deino-integrations

## Role

The integrations framework (INT-00, [ADR 0029](../../docs/adr/0029-integrations-framework.md)).
Integrations are the one place data leaves the Mac for a service other than
the chosen LLM. They always go directly from this Mac to the provider's API,
with the user's own sign-in, and never through a Deino server.

## Owns

- The provider contract (`provider.rs`): `Integration` (a
  `ProviderDescriptor` plus `identify`, an overridable `classify` for
  provider-specific rate limits and errors, and `revoke`), and the capability
  traits `PostMessage`, `CreatePage`, `CreateTask`, `LogActivity` and
  `ReadContext` (`capability.rs`). `Action` is a write typed by its
  capability.
- `registry.rs`: **the one list of providers** (one line per provider).
  `SandboxIntegration` talks to nothing and is used in debug builds only.
  `ScriptedIntegration` (feature `test-support`) answers whatever a test
  scripts.
- `catalog.rs`: every planned provider and what it will do. Under D12,
  `IntegrationService::providers` lists them as not available, and Settings
  → Integrations shows them as WIP.
- OAuth (`oauth/`), all from scratch with no OAuth crate:
  - PKCE S256;
  - the authorization URL;
  - `LoopbackListener` on `127.0.0.1`, plus `[::1]` for `localhost`, with three
    redirect modes: any port, port-agnostic `localhost`, or the user app's
    fixed port;
  - the token endpoint: code exchange, refresh, and RFC 7009 revocation. A
    client secret goes in the form body or in a Basic header, when needed;
  - `AuthorizationFlow`;
  - a device authorization (RFC 8628) helper.
- Token paste: `TokenScheme` covers `Bearer`, a bare key (Linear, ClickUp),
  and Basic with a username (Jira's email and API token).
- `CredentialVault`: one Keychain item per connection
  (`integration.<provider>.<connection id>`), holding the access and refresh
  tokens or the pasted token and its username as one JSON value. The user's
  client secret is `integration.<provider>.client-secret`. Both use the
  `deino-secrets` `CredentialStore` port.
- `integration_connections` and `integration_connection_scopes`
  (`connection/`), and the delivery log `integration_deliveries`
  (`delivery/`), each behind a repository port with a SQLite implementation.
- `IntegrationService` (`service/`):
  - providers, connect, cancel, token paste, check, disconnect;
  - sessions with single-flight token refresh;
  - `deliver`: idempotent, paced by the per-connection `RateLimiter` (GCRA),
    retried with backoff, and refreshed once on a 401;
  - `targets` and `read_context`;
  - the user's app registration.
- The ports the app implements:
  - `Browser`;
  - `Egress`, with `NetEgress` over `deino-net`;
  - `OAuthApps`, over `Settings::integrations`;
  - `Sleeper`, with `ClockSleeper`.

  Their fakes, plus a local mock authorization server, are behind
  `test-support` (`fake.rs`, `testing/`).

## Does Not Own

- Provider adapters (Slack, Notion, Linear, ...): INT-01..07 add one module
  and one registry line each.
- The automations engine, its `Step` trait and its stage: AUT-01 wraps
  `IntegrationService::deliver` in a step and maps
  `IntegrationError::is_retryable` to the pipeline's retryable error.
- Where registrations are stored (`deino_core::settings::IntegrationSettings`),
  the network policy (`deino-net`) and IPC (the app's `features/integrations.rs`).

## Invariants

- No token ever reaches SQLite, the settings file, a log, an error message or
  the webview. The UI can paste a token and save a client secret, but it can
  never read either back. `Credential`, `SecretValue`, `ProviderApi`,
  `OAuthClient` and `Failure` print redacted.
- Every request goes through a `deino-net` client scoped to the provider's
  `ServiceId`, so host checks, the user's switch, offline mode and the egress
  log always apply.
  - Connecting switches the service on, and the last disconnect switches it
    off.
  - Offline mode fails a connect or a delivery before anything is sent, with
    a message that says so.
- A delivery is sent at most once per idempotency key. A key that already
  succeeded returns the first result.
- A connection refreshes one token at a time. The renewed pair is written in
  one Keychain write before it is used, because providers that rotate refresh
  tokens revoke the whole grant when a spent one is reused.
- The loopback listener binds only loopback addresses and serves one
  redirect. A request with the wrong `state` is answered and ignored.
- No raw `tokio::spawn`, no wall clock: time comes from `Clock`, and waits go
  through `Sleeper`.
