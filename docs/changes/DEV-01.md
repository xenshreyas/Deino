# DEV-01 · Outbound webhooks (and Zapier/Make)

## Progress log (PARITY_PLAN §7.1)
DEV-01 adds signed outbound webhooks ([ADR 2026-10-12-webhook-deliveries](../adr/2026-10-12-webhook-deliveries.md)).

- **Core:** `deino-core::webhooks`.
  - `WebhookService`: create (secret shown once), update, delete, rotate secret, `send_test`, deliveries, `accept(WebhookSignal)`, `deliver_payload`.
  - `Dispatcher` and `DispatchLoop`.
  - `SigningSecret`: Standard Webhooks `webhook-id`/`-timestamp`/`-signature`, plus hex `x-signature`.
  - `schedule`: 15 s timeout; 30 s→2 h backoff for 24 h; auto-disable after a day of failures or on 410.
  - `taps`: `ReadyObserver`, `TappedNotesWriter`, `TappedActionItemEvents`, `TappedInsightGenerator`.
  - `WebhookStep`, AUT-01's `Action::Webhook { webhookId, fields }`.
- **Tables:** `webhooks`, `webhook_deliveries` (migration `20261012150000_dev01_webhooks.sql`).
- **Plugin:** `webhooks`, with commands `list`, `create`, `update`, `delete`, `rotate_secret`, `send_test` and `deliveries`.
- **UI:** Settings → Developer → Webhooks (`features/webhooks/`). The automations builder's webhook step is now ready.
- **Setup:** `docs/setup/zapier-and-make.md`.

## Follow-ups (PARITY_PLAN §7.2)
- **DEV-02:** the payload reads the meeting through EXP-01's `ExportAssembler` (via AUT-01's `AssembledMeetings`). DEV-02's `deino-core::query` read layer has no transcript text or insights. If it gains them, consider one read path.
- **AUT-01:** the webhook step targets an endpoint from Settings → Developer → Webhooks. It does not carry its own URL and secret.
  - Its idempotency key is `run:<run>:<endpoint>`, so two webhook steps to the same endpoint in one automation post once.
  - A per-step URL (Circleback-style) would need a hidden endpoint per step.
- **NOT-02 / ORG-03:** `notes.edited` covers AI-notes edits (the enhanced-notes writer). It does not cover the user's own notepad (`NotesEvents::saved`), which Granola does not count either. Add a tap there if wanted.
- **Owner QA:** add a Zapier Catch Hook and a Make Custom webhook, then:
  - send a test from Settings → Developer → Webhooks, and confirm both receive it;
  - confirm `x-signature` checks in Make with `sha256(body; "hex"; secret)`.
- **App exit:** the dispatch loop is not cancelled at exit (there is no per-feature exit hook). Pending deliveries resume at the next launch, and an attempt cut off mid-flight is reset to pending by `DeliveryRepo::recover`.

## Architecture (ARCHITECTURE.md)
- **Crates / `deino-core`:** add `webhooks` (DEV-01, ADR 2026-10-12-webhook-deliveries).
  - Endpoints and their Keychain secrets.
  - Signals tapped from the lifecycle, enhanced-notes writer, action-item events and insight generator.
  - A durable delivery queue with Standard Webhooks + `x-signature` signing, 24 h backoff and auto-disable.
  - AUT-01's webhook step.
- **Network:** `ServiceId::Webhooks` has no registry hosts. `SettingsServicePolicy::add_host_source` lets a feature add hosts from its own data, and the webhooks wiring adds every saved endpoint's host. The first endpoint switches the service on, and deleting the last switches it off.
- **Composition:** `composition/webhooks.rs` registers two attach hooks.
  - `webhooks` (Local 5): the service, plus `WebhookTaps` (the signal queue).
  - `webhooks-loop` (Background 70): the lifecycle's `ReadyObserver` and the `DispatchLoop`.
  - `action_items.rs`, `insights.rs` and `enhance.rs` (`NotesAi` binds a `LateSink` in `manage`) tap their event ports. `automations.rs` registers `WebhookStep`.

## CLAUDE.md
- **Crate table, `deino-core` row:** add `webhooks` (DEV-01: signed outbound webhooks, event-port taps, the durable delivery queue; ADR 2026-10-12-webhook-deliveries).
- **Patterns → Network:** a service whose hosts come from user data registers a `HostSource` with `SettingsServicePolicy::add_host_source` (webhooks do).

## Plan updates
- **DEV-01:** tick "Signature verification test vectors pass" (`webhooks::signing` tests, the Standard Webhooks spec vector) and "Retries are observed in tests" (`webhooks::dispatch_tests`, fake clock; `tests/webhooks_delivery.rs` end to end).
- **§7.2 rows this card closes:**
  - FND-12: webhook hosts are added through `SettingsServicePolicy`'s host source.
  - AUT-01: the webhook step is registered and ready.
