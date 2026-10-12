# CAP-13 · Consent and transparency tools

## Progress log (PARITY_PLAN §7.1)
| CAP-13 | Consent tools: the automatic chat message and the consent reminder. New crate `deino-consent` over CAP-05's window reader: one `ChatAdapter` per app (Zoom, Teams, Meet in Chrome) = its `SpeakerAdapter` + a `ChatVocabulary`; pure `decide` (transcribing, speech heard, empty composer, no key press for 3 s); `attempt::post` re-checks the typing guard before focus, write and Send and takes its own text back out; `Session` posts once per meeting; `Monitor`/`ChatNotice` poll the recorder every second; macOS `AxDriver` (`AXFocused`, `AXValue`, `AXPress`, no key events) and HID last-key-press. Settings `consent` (`chatMessage.{zoom,teams,meet,message}` off by default, `reminder` on). App: `composition/consent.rs`; plugin `consent` (`reminder`, `stop_reminding`, granted to the record panel). UI: `features/consent` (reminder card under the pill, Settings → Consent). Chat fixture dumps `chat-open-{empty,drafting}.json` per app. [ADR](../adr/2026-10-12-chat-notice-through-accessibility.md) |

## Follow-ups (PARITY_PLAN §7.2)
- Owner: **watermark camera** (CoreMediaIO camera extension overlaying "Transcribing"). Not built: it needs a system extension, an entitlement and signing; the card says to ask the owner before starting it.
- Owner QA: post the message on real **Zoom** and **Meet** test calls (and Teams), per `crates/deino-consent/README.md` → Manual QA; confirm nothing posts while typing, into a draft, or twice after pause/resume.
- Owner QA: replace the modelled `chat-open-*.json` dumps with real captures (`crates/deino-speaker-tags/fixtures/README.md` → Chat dumps), then adjust `ChatVocabulary` in `crates/deino-consent/src/apps.rs` if a test fails.
- Owner QA: Zoom keeps the chat's last recipient; check the notice goes to Everyone after a direct message, and decide whether to select "Everyone" first.
- CAP-13 (later): "once per meeting" is in memory; a restart of Deino mid-call followed by Resume could post a second time. Persist a sent flag per meeting if that matters.
- CAP-13 (later): Granola's extras are not built: Slack huddles, a late-joiner reminder, a live page of who is transcribing, pinning the message in Meet.

## Architecture (ARCHITECTURE.md)
- Rust workspace: add `crates/deino-consent` — the consent tools' chat notice (CAP-13): `ChatAdapter` per app over CAP-05's `SpeakerAdapter` and `WindowReader`, pure `decide` and `attempt::post` with the typing guard, `Session` (once per meeting), `Monitor`/`ChatNotice` (1 s poll of the recorder), macOS `AxDriver` and HID key activity; no network.
- App: `composition/consent.rs` (Background attach: `AppRecorder` over the playhead and segment count, `SettingsNotice` over `consent.chatMessage`); `features/consent` plugin (`reminder`, `stop_reminding`), granted to the `record_panel` window.
- Frontend: `features/consent` — `ConsentReminder` under the pill in `RecordPanel`; Settings → Consent page.

## CLAUDE.md
- Rust workspace table: `| crates/deino-consent | Consent tools (CAP-13): the automatic chat message in Zoom, Teams and Meet through Accessibility, over deino-speaker-tags' window reader; never while the user types (3 s since the last key press), never into a draft, once per meeting; tested on fixture dumps; no network |`

## Plan updates
- CAP-13: tick "The message never sends while the user is typing" (unit-tested: `gate::tests::it_never_acts_while_the_user_is_typing`, `tests/notice.rs` typing cases). "Manual QA posts the message in Zoom and Meet test calls" stays open for the owner.
- §5.1 row "Consent tools": Deino today → 🟡 (chat message and reminder; watermark camera with the owner).
