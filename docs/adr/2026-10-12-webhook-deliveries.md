# Webhooks: a durable delivery queue of their own, signed both ways, fed by taps on the event ports

- **Status:** Accepted
- **Date:** 2026-10-12
- **Card:** DEV-01 (Outbound webhooks, and Zapier/Make)

## Context

DEV-01 posts meetings to URLs the user enters: Zapier's "Catch Hook",
Make's "Custom webhook", or their own server. Granola (§9.3) signs with
Standard Webhooks, retries for days and turns a failing endpoint off;
Circleback (§12.4) sends the whole meeting with the fields the user picks,
signs the body with `x-signature`, and has "Send test request". The card
asks for both signatures, six events emitted from the services (no
polling), a 15 s timeout, exponential backoff for 24 h that survives a
restart, a delivery log, auto-disable with a notice, and AUT-01's webhook
step.

ADR 0031 already has idempotent deliveries (`IntegrationService::deliver`),
and the brief asked to reuse them if they fit. They do not:

- A delivery there belongs to a **connection** (an OAuth or pasted-token
  account with a credential, a session and a provider that classifies
  errors). A webhook endpoint has none of these: no account, no token to
  refresh, nothing to classify beyond the HTTP status.
- Its retries are **in the call**: three tries over about six seconds, then
  `failed`, left to the FND-08 runner's backoff through `is_retryable`. That
  runner is meeting-scoped and ordered, and `notes.edited` or
  `action_item.updated` are not pipeline work. Twenty attempts spread over a
  day, across restarts, need a stored due time, which ADR 0031's table does
  not have.
- Its rows record what was sent to whom; ours must also hold the exact body
  between retries, so every retry carries the same `webhook-id` and bytes.

## Decision

**Webhooks keep their own queue, in `deino-core::webhooks`.**
`webhook_deliveries` is one row per message to one endpoint across all its
attempts: status (`pending`, `sending`, `succeeded`, `failed`), attempts,
`next_attempt_at`, the last status and user-safe error, and the body. What
it shares with ADR 0031 is the idea, not the code: a caller key
(`idempotency_key`, unique per endpoint) makes AUT-01's step post once per
run, and a claim (`pending` → `sending`, compare-and-set) makes one
attempt at a time. A `sending` row left by a quit goes back to `pending` at
launch.

**One background loop, no polling of the services.** Event ports are
synchronous, so each is wrapped by a tap (`webhooks::taps`) that calls the
port and then hands a `WebhookSignal` to a queue: the meetings lifecycle's
observer (`processing` → `ready` is `meeting.ready`), the enhanced-notes
writer (version 1 is `notes.generated`; later versions and saved edits are
`notes.edited`), `ActionItemEvents` (created; updated, completed and
reopened are `action_item.updated`) and the `InsightGenerator` (a run that
answered is `insight.generated`). The `DispatchLoop` drains the queue into
deliveries for every enabled endpoint subscribed to the event and sleeps
until the next delivery is due. `notes.edited` and `action_item.updated`
**coalesce**: a new one joins a delivery of the same event and subject that
has not been attempted yet, due 30 s after the first, so autosaves send one
request.

**The body is fixed at the first attempt.** An event delivery is queued
without a body; the first attempt assembles the meeting as it is then
(EXP-01's assembler, so NOT-09 redaction applies) and stores it, and every
retry resends the same bytes with the same `webhook-id`. The body is cleared
once the delivery succeeds or fails, so the log keeps only metadata. An
automation step supplies its body (`deino.automation` v1, with the earlier
steps' outputs) when it queues the delivery.

**Schedule.** An attempt has 15 s. A network error, a timeout, 408, 429 or
5xx is retried after 30 s, doubling to at most 2 h, until the delivery is
24 h old (19 attempts); 3xx and other 4xx fail at once. Offline mode or a
switched-off service is retried but never counts against the endpoint. An
endpoint is turned off (`disabled_reason = failing`) once its attempts have
failed in a row for 24 h and at least five times, or at once on 410 Gone;
its waiting deliveries fail, and the page shows why until the user turns it
on again, which clears the streak. A "Send test" is attempted once and not
retried.

**Signing, both ways, from one secret.** The secret is `whsec_` and 32
random bytes in base64, shown once and kept in the Keychain
(`integration.webhook.<id>`, the `CredentialStore` of ADR 0031). Each attempt
carries Standard Webhooks' `webhook-id` (`msg_` and the delivery's UUID),
`webhook-timestamp` and `webhook-signature: v1,<base64 HMAC-SHA256 of
id.timestamp.body>` keyed with the decoded secret, checked against the
spec's test vector; and `x-signature`, the hex HMAC-SHA256 of the body keyed
with the secret exactly as shown, which a no-code tool can check without
base64.

**Network.** Requests go through `net.scoped(ServiceId::Webhooks)`. The
service has no registry hosts; the app's policy allows the hosts of the
saved endpoints (`WebhookService::hosts`) through a host source the
webhooks wiring registers on `SettingsServicePolicy`, so a host is
reachable only while an endpoint names it. The first endpoint switches the
service on and the last one's removal switches it off, as connecting an
integration does. URLs must be `https`, or `http` on a loopback host for
testing.

## Alternatives

- **`IntegrationService::deliver` with a "webhook" provider.** Needs a fake
  connection and credential per endpoint, and its in-call retries do not
  survive a restart; see Context.
- **Retries as FND-08 jobs.** The runner is per meeting and ordered; a
  failing endpoint would hold up a meeting's pipeline, and edits are not
  pipeline work.
- **Assemble the body at every attempt.** Simpler storage, but a retry could
  send different bytes under the same `webhook-id`, which breaks receivers
  that de-duplicate.
- **Store every body for the log.** Keeps a second copy of transcripts on
  disk for no reader; the log needs only the outcome.
- **Polling for changes.** The card forbids it, and taps cost one line per
  port at the composition root.

## Consequences

- A new event is a `WebhookEvent` variant, a tap on its port, and the
  migration's `CHECK` list.
- AUT-01's webhook step targets an endpoint from Settings → Developer →
  Webhooks rather than carrying its own URL and secret: one place to rotate
  secrets, see the log and get auto-disabled.
- The step's key is `run:<run>:<endpoint>`, so two webhook steps to the same
  endpoint in one automation post once.
- Deliveries happen only while Deino runs; a Mac asleep for a day loses
  retries but still makes at least one attempt per event.
