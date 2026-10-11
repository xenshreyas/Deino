# OAuth constraints per provider (research 2026-10-11)

INT-00's framework ([ADR 0031](../adr/0031-integrations-framework.md)) is shaped by these findings.

**Redirects:**
- Google accepts `127.0.0.1` on any port.
- Microsoft ignores the port on `http://localhost`.
- Slack, Linear and Salesforce want the exact port the user's app registered.

**Secrets:** Notion, Asana, HubSpot, Jira, ClickUp and monday.com need a confidential secret. For them, token paste is the default.

**Device flow:** only Microsoft (and Google, for limited-input devices) documents the device flow (RFC 8628).

The two passes below come from separate research sessions. Where they differ, the provider's own docs decide.

## Slack
- Auth method: PKCE (S256 only), generally available since 2026-03-30. Enabled per app, irreversible. The token exchange omits `client_secret`.
- Redirect: a custom scheme with PKCE, or `http://localhost:<FIXED port>` as a "desktop redirect".
  - The port must match the registered one exactly.
  - **Desktop redirects cannot request bot scopes; only `user_scope` works.**
- Tokens:
  - Access tokens rotate every 12 h.
  - Refresh tokens are single-use and expire after 30 days with PKCE.
  - Refreshing needs no secret.
- Revoke: `auth.revoke`.
- Token paste: no personal access token. The user must create an app and paste its xoxp token.
- Rate limits: tiered per method. On 429, honour the `Retry-After` header (seconds).
- **Recommended:** custom scheme + PKCE. Fallback: loopback on a fixed port.

## Notion
- Auth method: no PKCE. The token endpoint requires Basic `client_id:client_secret`, and so do revoke (`POST /v1/oauth/revoke`) and introspect.
- Tokens: the response has no `expires_in`. A refresh returns a new pair.
- Token paste:
  - Personal access tokens (2026) from the Developer portal. Business/Enterprise owners must allow them.
  - Or internal connection tokens. Pages must be shared with the connection.
- Rate limits: 180 requests/min per connection (600 on Business/Enterprise). Returns 429 `rate_limited` with `Retry-After`.
- **Recommended:** token paste (personal access token).

## Linear
- Auth method: PKCE (S256 or plain). The secret is optional, and a refresh needs only `client_id`.
- Redirect: `http://localhost:3000/...`-style. Assume an exact match on the port; custom schemes are reportedly rejected.
- Tokens:
  - Access tokens last 24 h.
  - Refresh tokens rotate on each refresh (30-minute grace). Mandatory since 2026-04-01.
- Revoke: `POST https://api.linear.app/oauth/revoke` (`token=`).
- Token paste: a personal API key, sent as `Authorization: <key>` with **no Bearer** prefix.
- Rate limits:
  - OAuth apps: 5,000 requests/h, 2M complexity points.
  - API keys: 2,500 requests/h, 3M complexity points.
  - **Rate limited = HTTP 400 `RATELIMITED`**, not 429. Use the `X-RateLimit-*` headers; there is no `Retry-After`.
- **Recommended:** loopback + PKCE on a fixed registered port.

## Asana
- Auth method: PKCE is listed, but token, refresh and revoke all require `client_secret`.
- Redirect: HTTPS only. Native apps get `urn:ietf:wg:oauth:2.0:oob` (copy the code). Localhost is rejected.
- Tokens: access tokens last 1 h; refresh tokens are long-lived.
- Revoke: `POST https://app.asana.com/-/oauth_revoke`.
- Token paste: a personal access token from https://app.asana.com/0/my-apps. It doesn't expire by default.
- Rate limits: 150 requests/min (free) or 1,500 (paid). Returns 429 with `Retry-After`. At most 50 GETs and 15 writes concurrently.
- **Recommended:** token paste (personal access token).

## Docs
- Slack: https://docs.slack.dev/authentication/using-pkce · https://docs.slack.dev/apis/web-api/rate-limits
- Notion: https://developers.notion.com/guides/get-started/personal-access-tokens · https://developers.notion.com/reference/request-limits
- Linear: https://linear.app/developers/oauth-2-0-authentication · https://linear.app/developers/rate-limiting
- Asana: https://developers.asana.com/docs/oauth · https://developers.asana.com/docs/rate-limits

## Google (Gmail, Drive, Docs, Calendar; "Desktop app" client)

**Auth method:** loopback + PKCE (S256).
- Redirect to `http://127.0.0.1:{RANDOM port}` or `[::1]`.
- Custom schemes and OOB are no longer supported.
- `client_secret` is marked optional, but Desktop clients reportedly still need it, so embed it. Google treats it as non-confidential.

**Tokens**
- Access tokens last about 1 h.
- Refresh tokens:
  - expire after 7 days while the app is in Testing status;
  - die after 6 months unused, or on a password change if the token has Gmail scopes;
  - are capped at 100 per account per client.

**Restricted scopes:** Gmail read/modify/compose and full Drive require verification plus a CASA assessment.

**Other**
- Revoke: `https://oauth2.googleapis.com/revoke`.
- No personal access token.
- Device flow can't get Gmail or Calendar scopes.
- Gmail quota: 6,000 units per user per minute. Retry with backoff.

## Microsoft Graph (public client)

**Auth method:** PKCE with no secret (forbidden for public clients).
- Redirect to `http://localhost`; the **port is ignored** (RFC 8252 §7.3).
- `127.0.0.1` is only possible through the manifest. No IPv6.

**Tokens**
- `offline_access` is required for refresh tokens.
- Refresh tokens last 90 days and are reissued on every use.

**Other**
- No standard revoke endpoint.
- Device code flow is supported.
- Outlook limits: 10k requests per 10 minutes and 4 concurrent requests per mailbox. Over the limit: 429 with `Retry-After`.

## HubSpot

**Auth method:** OAuth needs the secret and has no PKCE.
- Redirects must be https, or http localhost for testing only. IP addresses are rejected.

**Tokens**
- Access tokens last 30 minutes.
- Revoke: `POST /oauth/2026-03/token/revoke`.

**Token paste:** Service Keys (beta, admin only). New legacy private app tokens are sunset from 2026-10-26.

**Rate limits:** 110 requests per 10 s. Over the limit: 429 `RATE_LIMIT` plus `X-HubSpot-RateLimit-*` headers.

**Recommended:** token paste (Service Key).

## Attio

**Auth method:** OAuth needs the secret. PKCE is required for user tokens. The redirect must match exactly; the docs' example uses localhost:3050.

**Undocumented:** refresh tokens and a revoke endpoint.

**Token paste:** workspace API key (admin only, doesn't expire).

**Rate limits:** 100 reads and 25 writes per second. Over the limit: 429 with a `Retry-After` date.

**Recommended:** token paste.

## Salesforce
- **App registration:**
  - New Connected Apps are blocked (Spring '26). Use an External Client App instead; it must be Packaged to work in other orgs.
  - Since Sept 2025, users can't authorize an app their admin hasn't installed.
- **OAuth:** PKCE with no secret is supported (the Data Loader pattern). Redirect to `localhost:<fixed port>` or a custom scheme.
- **Tokens:**
  - Refresh tokens need the `refresh_token` scope.
  - Revoke at `{MyDomain}/services/oauth2/revoke`.
- **Rate limits:** a daily API allocation. When it runs out, the API returns **403 REQUEST_LIMIT_EXCEEDED**.
- **Recommended:** the user's own ECA. The user pastes the consumer key, PKCE is on, and the port is fixed.

## Todoist
- **OAuth:** public clients through dynamic registration (`token_endpoint_auth_method: none`) with PKCE and no secret. Redirects must be HTTPS, but localhost is allowed for testing; production use is unclear.
- **Tokens:**
  - Access tokens last 1 h.
  - Refresh tokens rotate. **Reusing an old one after 60 s revokes all of the user's tokens.**
  - Revoke at `/api/v1/revoke`.
- **Personal token:** Settings → Integrations → Developer.
- **Rate limits:** 429 with `retry_after`.
- **Recommended:** loopback + PKCE as a public client, with token paste as the fallback.

## Jira Cloud
- **OAuth:** no PKCE. The secret is required. Redirects work on `localhost:<port>`.
- **Tokens:**
  - Refresh tokens rotate (`offline_access`) and expire after 90 days of inactivity.
  - No revoke endpoint.
- **API token:** basic auth with `email:token`, valid for 1–365 days. Scoped tokens go through `api.atlassian.com/ex/jira/{cloudId}`.
- **Rate limits:** 429 with `Retry-After` and `RateLimit-Reason`.
- **Recommended:** token paste (API token plus email).

## monday.com
- **OAuth:** OAuth 2.1 with PKCE, but the secret is still required, including to revoke.
- **Personal API token:** found under Developers → API token. It doesn't expire.
- **Rate limits:** budgets based on query complexity, with `Retry-After` / `retry_in_seconds`.
- **Recommended:** token paste.

## ClickUp
- **OAuth:** needs the client secret. No PKCE. The access token never expires and there is no refresh.
- **PAT:** `pk_...`, sent in the `Authorization` header with **no Bearer prefix**.
- **Rate limit:** 100 req/min. A 429 carries `X-RateLimit-Reset`, a Unix timestamp; there is no Retry-After.
- **Recommended:** token paste.

## Trello
- **OAuth2:** Atlassian 3LO went GA on 2026-09-15. Its public-client and loopback rules are unclear.
- **Legacy flow:** `/1/authorize` with `return_url=http://localhost:<port>` (it must be in Allowed Origins) and `callback_method=fragment`. The token comes back in the URL fragment, so the loopback page needs a JS hop to post it to the listener.
- **Token paste:** API key + token.
- **Rate limit:** a 429 carries `API_TOKEN_LIMIT_EXCEEDED` or `API_KEY_LIMIT_EXCEEDED`; there is no Retry-After.
- **Recommended:** token paste first; the legacy loopback flow is optional.

## Affinity
- **OAuth:** none for the REST API.
- **API key:** only users whose admin grants the key permission can generate one. Use Bearer for v2.
- **Rate limit:** 900 req/min per user. A 429 carries `X-Ratelimit-*`.
- **Recommended:** token paste.

## Zoho CRM
- **OAuth:** PKCE, but the secret is still required, so it is embedded in the app.
- **Redirect:** a fixed registered port. Pass `access_type=offline`.
- **Refresh tokens:** never expire and are not rotated. The cap is 20 per user; minting a 21st deletes the oldest.
- **Multi-DC:** use the `accounts-server` from the callback and the `api_domain` from the token response.
- **Device grant:** exists.
- **Rate limit:** a daily credit budget. Overruns come back as a `TOO_MANY_REQUESTS` error code, not HTTP 429.
- **Recommended:** loopback with the embedded secret and multi-DC support, or the user's own Self Client.

## Second pass (16 providers, official docs, 2026-10-11)

"Unclear" means the official docs read did not say.

| Provider | PKCE without a secret | Redirect rules | Refresh | Revoke | Paste alternative | Recommended |
|---|---|---|---|---|---|---|
| Slack | Yes (S256, opt-in, permanent) | Custom scheme, or `http://localhost` desktop redirect matching the registered port; user scopes only | 30 days with PKCE; rotates | `auth.revoke` | User's own app token (xoxp) | Custom scheme; fallback loopback on a fixed port |
| Notion | No (Basic client_secret) | Unclear | Rotates (current and previous valid) | `POST /v1/oauth/revoke` | Internal token, PAT | Token paste |
| Linear | Yes (secret optional) | `http://localhost:3000`-style; port matching unclear | 24 h access, rotating refresh (30 min grace) | `POST /oauth/revoke` | Personal API key, no `Bearer` | Loopback + PKCE on a fixed port |
| Google (Desktop) | Yes; Desktop secret treated as non-confidential | `http://127.0.0.1:<random>`; custom schemes removed | Long-lived; count limits | `oauth2.googleapis.com/revoke` | None | Loopback + PKCE, random port |
| Microsoft | Yes, no secret | `http://localhost`, port ignored; 127.0.0.1 via manifest; no `[::1]` | 90 days, replaced on every use | None (RFC 7009) | None | Loopback + PKCE |
| HubSpot | No (secret) | https, or http localhost for testing; no IP addresses | Short-lived access | `/oauth/2026-03/token/revoke` (secret) | Static-auth app token / Service Key | Token paste |
| Attio | PKCE required for user tokens; secret unclear | `localhost:3050` in tutorial | Unclear | Unclear | Workspace API key | Token paste |
| Asana | No (secret required) | https, or `urn:ietf:wg:oauth:2.0:oob` | 1 h access | `/-/oauth_revoke` (secret) | PAT | Token paste |
| Todoist | Only for dynamically registered public clients | https; localhost "for testing" | 1 h access; rotating; reuse after 60 s revokes all | `POST /api/v1/revoke` | Personal API token | Token paste |
| Jira Cloud | No (secret; no PKCE) | Must match | Rotates; 90 days idle | None | Email + API token (Basic) | Token paste (two fields) |
| Salesforce | Yes via External Client App with secret off | `http://localhost:<port>` matching registration | Rotation on by default, reuse detection | `/services/oauth2/revoke` | None | User's own ECA, loopback + PKCE, fixed port |
| monday.com | PKCE required, secret still sent on refresh | Must match | Rotating | `auth.monday.com/oauth_ms/oauth/revoke` | Personal API token | Token paste |
| ClickUp | No (secret) | Non-SSL "may not be supported" | Tokens do not expire | None | `pk_` token, bare header | Token paste |
| Trello | No OAuth 2 (1/authorize with API key) | Token in URL fragment (needs a relay page) | Chosen lifetime, no refresh | `DELETE /1/tokens/{token}` | Pasted token | Token paste; later a fragment-relay page |
| Affinity | No OAuth for REST | n/a | n/a | n/a | API key (Bearer, v2) | Token paste |
| Zoho CRM | Public mobile/desktop client must use PKCE; secret unclear | Custom scheme; localhost unclear | Unlimited until revoked | `{accounts-domain}/oauth/v2/token/revoke` | Self Client (needs secret) | Custom scheme; fallback user's own Self Client |

Docs: docs.slack.dev/authentication/using-pkce, developers.notion.com/docs/authorization,
linear.app/developers/oauth-2-0-authentication,
developers.google.com/identity/protocols/oauth2/native-app,
learn.microsoft.com/entra/identity-platform/reply-url,
developers.hubspot.com/docs/apps/developer-platform/build-apps/authentication/oauth/working-with-oauth,
docs.attio.com/rest-api/guides/authentication, developers.asana.com/docs/oauth,
developer.todoist.com/api/v1/, developer.atlassian.com/cloud/jira/platform/oauth-2-3lo-apps/,
help.salesforce.com (External Client App OAuth settings), developer.monday.com/apps/docs/oauth,
developer.clickup.com/docs/authentication,
developer.atlassian.com/cloud/trello/guides/rest-api/authorization/,
developer.affinity.co/pages/external-api-v2/authentication,
zoho.com/accounts/protocol/oauth/mobile-applications.html.
