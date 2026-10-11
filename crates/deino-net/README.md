# deino-net

## Role

The only way Deino talks to the internet (FND-12, [ADR 0019](../../docs/adr/0019-network-egress-policy.md)).
Every outgoing request is confined to one named service, allowed only while
the user allows that service, stopped by offline mode, and recorded in the
egress log the user can read in Settings → Privacy → Network.

## Owns

- `NetClient`: one required `send(Request) -> Response`, with `fetch` (whole
  body), `events` (Server-Sent Events) and, on `NetClientExt`, JSON helpers.
  Deino's own `Request`, `Response`, `Method`, `Status` and `Headers`, so no
  caller sees `reqwest`.
- `ReqwestNetClient`: the workspace's only HTTP client (native TLS, no
  automatic redirects). `FakeNetClient` and `FakeServicePolicy` behind
  `test-support`.
- `registry`: the hosts and defaults of every `deino_domain::ServiceId`.
  `HostPattern::AnyPublicName` (company logos, `favicons`, ORG-04) admits any
  public DNS name and never an IP address, `localhost` or a private suffix.
- `Net` and `ScopedClient`: `net.scoped(service)` reaches only that service's
  hosts; `net.approved_download(ServiceId::HfModels)` also works in offline
  mode. Redirects are followed here, one checked and logged hop at a time.
- The `ServicePolicy` / `ServiceSwitch` ports (the app implements them over
  the `network` settings section).
- `EgressLog` (in-memory ring), `EgressPersister`, and `SqliteEgressLog` over
  the `egress_log` table.
- `Downloader` / `HttpDownloader`: resumable `Range` downloads into
  `<file>.part`, a size cap, a stall timeout, SHA-256 verification and an
  atomic rename.
- `NetworkControls`: what the Network page lists and switches.

## Does Not Own

- Where the user's choices are stored: `deino_core::settings::NetworkSettings`
  (`network.offlineMode`, `network.services`), read through `ServicePolicy`.
- Provider protocols (LLM request shapes, OAuth): `deino-llm` (FND-09) and the
  integration cards, which receive a `ScopedClient`, never this crate's root.
- IPC: the app's `features/network.rs`.
- Model catalogs (which file, which hash): the model managers.

## Invariants

- No other crate depends on `reqwest` or another HTTP client;
  `deino-audio` and `deino-stt` never depend on this crate
  (`scripts/check-dependency-rules.sh`, the gate's `deps` step). clippy's
  `disallowed-types` bans `reqwest::Client` outside `reqwest_client.rs`.
- A request to a host outside its service fails before anything is sent, and
  so does every redirect hop. A cross-origin hop drops credentials; an
  `https` request is never downgraded.
- Offline mode blocks everything except downloads the user started.
- An egress entry has no field for a path, a query string, a header or a
  body; `Request`'s `Debug` prints only the method and host; transport
  errors are stripped of their URL before they are logged.
- A downloaded file reaches its destination only after its size and SHA-256
  check out; an interrupted download keeps its partial file for a resume.
