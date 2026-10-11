# deino-connect

## Role

Deino's connections to the user's online accounts (ORG-01,
[ADR 0027](../../docs/adr/0027-calendar-sync-through-eventkit.md)): sign in
with OAuth on this Mac, then read through `deino-net`. It exists so that
`deino-core` never depends on a network client.

## Owns

- `oauth`: OAuth 2.0 for native apps (RFC 8252) with PKCE S256 (RFC 7636).
  `OAuthProvider` (`GOOGLE`, `MICROSOFT`: endpoints, read-only scopes,
  redirect host), `OAuthClient` (authorization URL, code and refresh grants),
  `LoopbackRedirect` (a one-shot listener on `127.0.0.1` that checks
  `state`), `sign_in` (browser, redirect, exchange, 5-minute limit),
  `TokenSet`, `BrowserOpener` (the port the app implements with
  `NSWorkspace`). Provider-neutral: INT-00 adds Gmail, Drive and the other
  integrations here.
- `calendar`: `ConnectorSource`, a `deino_core::calendar::CalendarSource`
  over a `CalendarApi` (`GoogleCalendar`, `MicrosoftCalendar`), with their
  wire types and the mapping into the core's `SourceSnapshot`;
  `SettingsCredentials` (client ID from `Settings::calendar`, Google's client
  secret from the Keychain).
- `FakeBrowser` (feature `test-support`).

## Does Not Own

- Calendar rules (join links, external, filters, series), storage and
  scheduling: `deino_core::calendar`.
- The HTTP client, host lists, network switches and the egress log:
  `deino-net` (`ServiceId::GoogleCalendar`, `ServiceId::MicrosoftGraph`).
- macOS Calendar: `deino-eventkit`.
- Client registration: the owner's (`docs/setup/google-calendar.md`,
  `docs/setup/microsoft-graph.md`).

## Invariants

- No `tauri`, no HTTP client of its own: every request goes through a
  `deino-net` scoped client, so offline mode and a switched-off service send
  nothing.
- Refresh tokens (and Google's client secret) live only in the Keychain;
  access tokens only in memory. Neither is logged, nor are event titles,
  descriptions or attendees.
- Connecting switches the provider's service on; disconnecting forgets the
  refresh token and switches it off.
- The loopback listener binds `127.0.0.1` only, accepts one redirect whose
  `state` matches, and closes.
