# deino-connect

## Role

Deino's connections to the user's online calendar accounts (ORG-01,
[ADR 0027](../../docs/adr/0027-calendar-sync-through-eventkit.md)). It signs in
with OAuth on this Mac, then reads through `deino-net`. The crate exists so
that `deino-core` never depends on a network client.

## Owns

- `calendar`:
  - `ConnectorSource`, a `deino_core::calendar::CalendarSource` over a
    `CalendarApi` (`GoogleCalendar`, `MicrosoftCalendar`), with their wire
    types and the mapping into the core's `SourceSnapshot`;
  - `calendar/oauth.rs`: the two providers' `OAuthSpec`s (endpoints,
    read-only scopes, redirect rules, client authentication);
  - `SettingsCredentials`: the client ID from `Settings::calendar`, and
    Google's client secret from the Keychain.

## Does Not Own

- OAuth itself: PKCE, the loopback listener, the token endpoint, the
  `Browser` port and its fake. These belong to `deino_integrations::oauth`
  (INT-00, [ADR 0029](../../docs/adr/0029-integrations-framework.md)), the
  one OAuth layer in Deino.
- Calendar rules (join links, external, filters, series), storage and
  scheduling: `deino_core::calendar`.
- The HTTP client, host lists, network switches and the egress log:
  `deino-net` (`ServiceId::GoogleCalendar`, `ServiceId::MicrosoftGraph`).
- macOS Calendar: `deino-eventkit`.
- Client registration: the owner's (`docs/setup/google-calendar.md`,
  `docs/setup/microsoft-graph.md`).

## Invariants

- No `tauri`, and no HTTP client of its own. Every request goes through a
  `deino-net` scoped client, so offline mode and a switched-off service send
  nothing.
- Refresh tokens (and Google's client secret) live only in the Keychain.
  Access tokens live only in memory. None of them is logged, and neither are
  event titles, descriptions or attendees.
- Refreshes never overlap: the access-token cache lock is held across them.
  A rotated refresh token is stored before the new access token is used.
- Connecting switches the provider's service on. Disconnecting forgets the
  refresh token and switches the service off.
