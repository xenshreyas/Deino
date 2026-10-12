# 0043. Redact secrets by pattern, with an optional check on the local model only

- **Status:** Accepted
- **Date:** 2026-10-11
- **Card:** NOT-09 (sensitive-info redaction)

## Context

People read passwords, card numbers and one-time codes aloud in meetings.
NOT-09 adds an optional setting, off by default, that keeps them out of the
enhanced notes and exports. The card asks for patterns plus "a light LLM
check", with `[redacted]` in place of each secret and the originals kept in
the transcript.

The notes are usually written by a hosted model (§8 D3: Claude when a key
is set). The point of redaction is that a secret never reaches it. So the
check cannot be "ask the notes model whether this is a password".

## Decision

**Patterns decide; a model may only narrow.** `deino_core::redaction` runs
one `Detector` per `SecretKind` (password, API key, card number, SSN,
one-time code) over a `Scan` of the text: its words with byte ranges and its
digit runs, typed (`4242 4242`, `123-45-6789`) or spoken ("four two four
two", "double four", "oh"). Each `Redaction` span carries a `Confidence`:

- **Certain** spans prove themselves by shape: 13–19 digits on a card
  network's prefix that pass Luhn, a provider key prefix (`sk-ant-`, `ghp_`,
  `AKIA` …), an SSN written 3-2-4 with issuable parts. They are always
  redacted and never shown to a model.
- **Likely** spans come from context: the words after "password is", 4–8
  digits after "code"/"PIN"/"OTP", nine digits after "social security",
  long mixed-case high-entropy tokens. Without a check they are redacted.

**The check runs only on the built-in local model.** `LocalModelCheck`
resolves the `tagging` route; when it is served by `ProviderId::BuiltIn` it
sends the likely candidates, each as a few words of context with the span
between « and » (never the transcript), and asks for `{number, secret}`
verdicts through `Gateway::structured`. Only an explicit "not a secret"
rejects; a missing verdict, a parse failure or an unavailable model keeps
the span. When anything else serves the route (Claude, an OpenAI-compatible
endpoint, Ollama, whose endpoint may be another machine), the check is
skipped and every candidate is redacted. Sending a
suspected password to a cloud model to decide whether to hide it from a
cloud model would defeat the purpose.

**Where it applies.**

- Notes input: `MaterialSources::gather` redacts the transcript lines in
  one batch (`with_redactor`), so `notes.enhance`, `title` and any later
  stage that gathers material there (AI-06 insights) see `[redacted]`.
  This path uses the local check.
- Exports, auto-export, the account zip and the `deino` CLI/MCP:
  `ExportPorts::with_redactor` wraps the notes (`RedactedNotes`) and
  redacts the transcript and action items in the assembler. These use
  patterns only: preview, copy and the CLI must not wait on a model, and
  over-redacting a copy loses nothing the transcript does not keep. The CLI
  reads the app's `settings.json` beside the database.
- The transcript, the transcript view and the citations popover keep the
  originals; the user deletes a secret there.

**One setting**, `privacy.redactSensitive` (Settings → Network → Sensitive
information), read at every call through the `RedactionSetting` port, so a
change applies to the next notes or export without a restart.

## Alternatives

- **Ask the notes model to leave secrets out** (Granola's "tries its best").
  The secret still reaches the provider; rejected.
- **A dedicated `redact.check` route, always local like `embed`.** Cleaner
  metering, but a new route widens `llm_calls.route`'s CHECK with a
  migration and adds a row to Settings → AI that cannot be changed. Reusing
  `tagging` (short local classification) is enough for an S card; a
  follow-up can add the route.
- **Run the check on hosted models too.** Defeats the purpose (above).
- **A redaction pipeline stage that writes redacted copies.** Two transcripts
  to keep in sync, and redaction would lag behind the setting. Redacting on
  read keeps one source of truth.
- **Named-entity models (Presidio-style).** A Python service or a large
  model for five kinds whose shapes are well known; rejected.

## Consequences

- With a hosted fast route, likely spans are always redacted: "the password
  is on the sticky note" loses its tail in the notes input. The user can
  route `tagging` to the built-in model to get the check.
- Years (1900–2099) are never one-time codes, so a four-digit PIN in that
  range said after "PIN" is missed; zip, postal, area and country codes
  after "code" are kept. Passwords run to the end of the clause, at most
  four words, so a longer spoken passphrase leaks its tail.
- The user's own typed notes reach the notes prompt unredacted (the
  provenance guard renders them verbatim); exports redact them.
- Exports and the CLI never call a model, so they may redact a span the
  notes kept after the local check.
