# 0019. Send every request through deino-net, scoped per service and logged

- **Status:** Accepted
- **Date:** 2026-10-09
- **Card:** FND-12 (`docs/references/architecture.md` (b) FND-12)

## Context

Deino promises that data leaves the Mac only for services the user connects
(PARITY_PLAN §1, D2): the LLM provider for notes and chat, and integrations.
Before FND-12, nine inherited modules each built their own `reqwest` client
(LLM providers, three model downloaders, a connection test), nothing knew
which hosts were legitimate, a redirect could go anywhere, the webview's CSP
let the UI call Ollama directly, and the user had no way to see or stop what
was sent. Transcription and audio code sat in the same crate as those clients.

## Decision

- **One crate owns HTTP.** `deino-net` is the only crate that depends on
  `reqwest`. Callers speak its own `Request`/`Response` through the
  `NetClient` trait (one required `send`, helpers as provided methods: the
  shape of Zed's `HttpClient`, reference only). clippy's `disallowed-types`
  bans `reqwest::Client` everywhere else, and
  `scripts/check-dependency-rules.sh` (the gate's `deps` step) fails if any
  other crate under `crates/` depends on an HTTP client, or if `deino-audio`
  or `deino-stt` depend, even indirectly, on `deino-net` or one.
- **Capabilities, not a shared client.** Every outside service is a
  `deino_domain::ServiceId` with registered hosts (`deino_net::registry`).
  Consumers get `net.scoped(service)`, a `ScopedClient` that can reach only
  that service's hosts (plus a custom endpoint from the settings for the
  services that take one). The root client never follows redirects; the
  scoped client follows them itself, checking and logging each hop, dropping
  credentials across origins and refusing `https` → `http`.
- **The user decides, per service.** Before each request the scoped client
  asks the `ServicePolicy` port, which the app implements over the typed
  settings (`network.services`, `network.offlineMode`). A service the user
  never switched keeps its registry default: on for the LLM providers and
  model downloads, which send nothing until the user sets them up (a key, an
  endpoint, a click), off for integrations, web search and webhooks until
  they are connected. A switch applies to the very next request.
- **Offline mode means nothing leaves.** It blocks every request, loopback
  included, except model downloads the user started
  (`net.approved_download(ServiceId::HfModels)`).
- **Everything is logged, nothing sensitive is.** Each attempt, including
  blocked ones and each redirect hop, becomes one `EgressEntry` (time,
  service, host and port, method, status, outcome, bytes up and down). It
  has no field for a URL path, query, header or body, and the table's
  `CHECK` refuses a host containing `/` or `?`. Entries live in a 500-entry
  ring and, once the database is open, in `egress_log` (newest 10,000 rows),
  written by one persister task so recording never blocks a request.
- **Downloads are verified.** `HttpDownloader` resumes `<file>.part` with
  `Range`, treats `416` with a matching total as complete, restarts when a
  server ignores the range, caps the size, times out a stalled body and
  checks the pinned size and SHA-256 before an atomic rename.
- **The UI has no network.** The CSP's `connect-src` is
  `'self' ipc: http://ipc.localhost asset:`; the dev CSP adds the Vite
  server (`ws:`/`http://localhost:3118`) for hot reload only.

## Alternatives

- **Grep for `reqwest` in CI.** Catches imports, not a dependency added
  through another crate, and says nothing about hosts, consent or logging.
- **One shared client with a host allowlist.** Any caller could reach any
  allowed host; a scoped client makes "the Slack integration talks to Slack"
  a property of the type a module is given.
- **Let reqwest follow redirects with a custom policy.** The policy is
  global to the client and cannot know which service a request is for.
- **Default every service to off.** Existing users who chose Claude would
  find notes broken after an update; LLM providers already require explicit
  setup, which is the consent.
- **Log URLs with the query stripped.** Paths still carry IDs (a Notion page,
  a Slack channel) and are not needed to answer "what left my Mac".

## Consequences

- New outbound code takes a `ScopedClient` (or `Arc<dyn NetClient>`) and
  adds its service to `ServiceId` and the registry. A new `ServiceId` (or
  `Method`) needs a migration that rebuilds `egress_log`, whose `CHECK` lists
  them.
- The LLM gateway (FND-09) gives each provider a `NetTransport` around its
  own scoped client (`LlmAnthropic`, `LlmOpenAi`, `LlmGroq`, `LlmOpenRouter`,
  `LlmCustom`, `LlmOllama`); a blocked call reaches the gateway as a
  connection error whose message says why (offline mode, a switched-off
  provider).
- Every model download (Parakeet Unified through `deino-core`'s
  `ByteSource`, Parakeet TDT, Whisper, the built-in LLM's GGUF) takes
  `net.approved_download(HfModels)` per call; the engines hold no client, so
  they appear in the egress log and work in offline mode.
- Two `reqwest` uses remain outside `deino-net`: the legacy
  `api_test_custom_openai_connection` command (it tests an endpoint before
  it is saved, which a scoped client would refuse; Settings → AI's route
  test replaces it) and `deino-llm`'s eval harness (a dev-dependency, never
  shipped). The legacy lint exemption keeps `disallowed_types` until the
  command is deleted.
