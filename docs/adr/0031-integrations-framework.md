# 0031. One integrations framework: capabilities, loopback OAuth, Keychain credentials, idempotent deliveries

- **Status:** Accepted
- **Date:** 2026-10-11
- **Card:** INT-00 (scope reduced by D12)

## Context

Integrations are the only data that leaves the Mac for a service other than
the chosen LLM. Deino has no server, so every integration talks straight
from the Mac to the provider's API with the user's own sign-in (PARITY_PLAN
§1, §4.5). Several later pieces need the same parts:

- seven provider cards (INT-01..07);
- the automations engine's actions (AUT-01);
- webhooks (DEV-01);
- MCP connectors (AI-07);
- the calendar connectors ORG-01 shipped in `deino-connect`.

The common parts are:

- a typed way to say what a provider can do;
- sign-in without a relay;
- tokens that never touch SQLite or the webview;
- refresh;
- rate limits and retries;
- writes that are never sent twice.

We researched each provider's OAuth rules from its official docs
([`docs/research/integrations-oauth.md`](../research/integrations-oauth.md)).
No two providers agree:

- **Google** wants `127.0.0.1` on any port, and sends a Desktop client
  secret that it treats as public.
- **Microsoft** wants `http://localhost` and ignores the port.
- **Slack and Linear** want the exact `localhost` port the user's app
  registered.
- **Notion, Asana and HubSpot** need a confidential secret.
- **Linear** signals rate limits as `400 RATELIMITED`, and **Salesforce** as
  `403 REQUEST_LIMIT_EXCEEDED`.
- **Todoist** revokes the whole grant when a rotated refresh token is
  reused.
- **Jira** pastes an email together with its token.
- **Zoho** names the account's API host in the token answer.

D12 (2026-10-11) deprioritized the direct integrations. INT-00 ships the
framework, the sandbox provider and the Settings page. Planned providers
appear as WIP.

## Decision

A new crate, `crates/deino-integrations`, sits below `deino-core`. It
depends on `domain`, `db`, `net` and `secrets`, so AUT-01 can use it from
the core.

- **Capability model.** Each provider implements `Integration`, which holds
  a `ProviderDescriptor` and `identify`.
  - `classify` and `revoke` are overridable.
  - Capabilities are small traits: `PostMessage`, `CreatePage`, `CreateTask`,
    `LogActivity` and `ReadContext`. A provider offers one by returning
    `Some(self)` from its accessor.
  - `Action` is a write whose payload type is fixed by its capability, so an
    automation step that targets "any provider that can create tasks" is
    type-checked.
  - `registry::integrations` is the one list.
  - `SandboxIntegration` talks to nothing and is registered in debug builds
    only. `ScriptedIntegration` (behind `test-support`) speaks real HTTP to a
    scripted transport.
- **Auth methods per provider.** Each provider declares one or more ways to
  connect. `OAuthSpec` covers:
  - the authorize, token and revoke URLs;
  - the scopes, each with a description shown to the user;
  - the scope parameter and separator;
  - extra parameters;
  - `RedirectSpec`: host `127.0.0.1` or `localhost`, any port or a fixed
    port the user can change;
  - `ClientAuth`: `Public`, `SecretInBody` or `SecretInBasicHeader`.

  `TokenSpec` covers a pasted token: its label, an optional username field,
  and a `TokenScheme` of `Bearer`, bare or Basic.
- **OAuth without a relay.** The OAuth code is written from scratch, with no
  OAuth crate:
  - PKCE S256 (checked against the RFC 7636 vector);
  - a one-shot `LoopbackListener` on loopback only. It checks `state`,
    answers stray requests and ignores them, and serves a short page;
  - the token endpoint: code exchange, refresh and RFC 7009 revocation;
  - `AuthorizationFlow`, the whole sign-in;
  - a device-code helper (RFC 8628). No provider uses it yet.

  Deino ships no client IDs or secrets. The user registers their own app:
  the client ID and port go in `Settings::integrations.apps`, and a secret
  goes in the Keychain.
- **Credentials.** `deino-secrets` gains `CredentialKey` and the
  `CredentialStore` port, for run-time Keychain accounts of the form
  `integration.<provider>.<connection>`. `KeychainSecretStore` implements
  both ports, and `InMemoryCredentialStore` is the fake.
  - A connection's whole credential is one JSON item: the access and refresh
    tokens, or the pasted token and its username. A rotated pair is
    therefore replaced in one write.
  - Tokens never reach SQLite, settings, logs or IPC responses.
- **Connections.**
  - `integration_connections` holds the provider, account ID and label, auth
    method, status, per-account API base URL, token expiry, timestamps and
    the last user-safe error. It has no secrets.
  - `integration_connection_scopes` holds one row per scope.
  - Reconnecting the same account keeps its ID.
- **Sessions.** An access token is refreshed two minutes before it expires,
  or once after a 401. Refreshes are single-flight per connection: under
  the lock, a caller that waited re-reads the tokens and the row and uses
  what another caller renewed. This is why a rotated refresh token is never
  reused.
- **Deliveries.** `integration_deliveries` has one row per caller-chosen
  `IdempotencyKey`. A compare-and-set claim lets only one caller send. A
  stale `sending` row, left by a crash, can be reclaimed after 10 minutes. A
  succeeded key returns its first result. Each call:
  - waits for a per-connection GCRA rate limiter;
  - makes at most 3 tries;
  - backs off 2 s and then 4 s, or honours `Retry-After` up to 60 s;
  - classifies failures through the provider.

  A delivery that still fails is left `failed`. `is_retryable` lets AUT-01's
  pipeline stage hand it to the FND-08 runner's backoff, and the key stops
  that from writing twice. Refresh is on demand, not a scheduled job.
- **Policy.** Every request goes through a `deino-net` client scoped to the
  provider's `ServiceId`. Six `ServiceId`s are added (Jira, monday.com,
  ClickUp, Trello, Affinity, Zoho CRM), which rebuilds `egress_log`.
  - Connecting switches the service on, and the last disconnect switches it
    off.
  - Offline mode fails a connect or a delivery before anything is sent. The
    delivery log records why, and the connection's error is left alone.
- **One OAuth layer.** ORG-01's `deino-connect` had its own OAuth module. Its
  Google and Microsoft calendar connectors now use
  `deino_integrations::oauth` and the shared `Browser` port, and the
  duplicate module is deleted. Calendar accounts stay `calendar_accounts`
  rows with their refresh tokens under `SecretName`; see Consequences.
- **IPC and UI.** The `integrations` plugin exposes these commands:
  `providers`, `connect`, `cancel_connect`, `connect_token`, `check`,
  `disconnect`, `set_app`, `remove_app`, `targets`, `deliver` and
  `deliveries`. Settings → Integrations lists:
  - the shipped providers, with Connect, their accounts, status, last use,
    scopes and last error;
  - the planned ones, with a WIP badge and a disabled Connect;
  - what was sent.

## Alternatives

- **The `oauth2` crate.** It is a good library, but the loopback listener,
  the provider quirks and the secret handling would still be ours. A
  dependency would add little.
- **A relay or a custom URL scheme everywhere.** A relay is ruled out by the
  product. A custom scheme needs DEV-05's `deino://` handler, and Google
  bans it. It stays a follow-up for Slack and Zoho.
- **One generic `Destination` trait with every method.** It cannot be
  substituted (most providers support two methods), and it hides from the
  type checker which provider can do what.
- **Tokens in SQLite, encrypted.** That needs a key, and the key would live
  in the Keychain anyway. The Keychain alone is simpler and is what FND-06
  chose.
- **Retries as FND-08 jobs.** The runner is meeting-scoped and ordered.
  Short in-call retries plus the stage's own retry cover it without a second
  queue.

## Consequences

- A provider card is one module plus one registry line. It moves the
  provider out of the planned catalogue and may override `classify`.
- Slack's PKCE returns user tokens under `authed_user`, so the INT-01
  adapter needs a token-response mapper. A custom-scheme redirect waits on
  DEV-05.
- Calendar accounts keep their own storage. Moving them to
  `integration_connections` would need an `IntegrationProvider` for each
  calendar provider, plus a migration of `calendar_accounts`. That is left
  as a follow-up.
- The page has no OAuth app form yet. `set_app` exists, but no shipped
  provider uses OAuth.
