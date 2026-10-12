# UX-settings · Settings: nine pages, one permission row, one AI section

UX-09, UX-10, UX-11, UX-13 to UX-20 of `docs/UX_SIMPLIFICATION.md`, with
UX-33 and UX-34 applied to every touched file. Frontend only; no settings
field, migration or Rust code changed.

## Progress log (PARITY_PLAN §7.1)

| Item | What shipped |
|---|---|
| UX-09 | Speaker names: no per-app permission rows. The section lists the apps as plain text (an app marked only when unsupported) and links to the one Accessibility row. |
| UX-10 | Settings → Recording → Permissions: microphone, system audio, notifications, Accessibility, Screen Recording and Calendar, once each, with onboarding's rows (`onboarding/components/permission-rows.tsx`, shared with the Permissions step). Detection, consent, screen capture and speaker names show `PermissionLink` ("Needs Accessibility") instead of their own rows; screen's `permission-card.tsx` and consent's permission row are deleted. |
| UX-11 | Notes → AI: onboarding's three choices and one key check (`onboarding/components/notes-provider.tsx`, shared with the AI step); Groq, OpenRouter, Ollama and custom endpoints under "Other providers" (the legacy model panel, `OtherProviders`). Per-task models: Advanced → AI per task, collapsed. Auto Summary left Notes; it is Advanced → Notes after meetings (see Follow-ups). |
| UX-13 | Recording: "Save audio recordings", "Recording start notification", "I use headphones" and the file-format box are gone (`components/RecordingSettings.tsx` deleted). Typed microphone select over `recording.microphones`. Fields kept: `recording.autoSave`, `recording.showRecordingNotice`, `audio.usesHeadphones`, `audio.preferredSystemDevice`. |
| UX-14 | Advanced → Network services lists only services in use (in the activity log, or with a saved switch); "Show all N services" reveals the rest. Offline mode and the log stay in Privacy. |
| UX-15 | `ui/more-options.tsx`. One primary control per feature, the rest closed: screen capture (snapshots, video, smoothness), auto-export (names, contents), detection (auto-start, countdown, ignored apps), auto-stop, consent's chat message, calendar auto-record rules. |
| UX-16 | Search maintenance is Advanced → Search, in plain words (no "index", "embeddings", "keyword index"). The download already starts the backfill. |
| UX-17 | General → Notifications: recording started and stopped, meeting reminders, action items due today; which calendars remind under More options. Keys unchanged. |
| UX-18 | People → Advanced → Voices (alias `people`); recognising speakers is a switch on Recording. Import and Watched folders sit together on Connections. Profile's "Context and glossary" is "Background" and points to Spelling. |
| UX-19 | Nine pages, no group headings (ADR `2026-10-12-settings-host-pages-and-sections`): General, Recording, Notes, Templates, Calendar, Connections, Privacy, Advanced, About. Features register sections onto host pages; every one of the 27 old page ids resolves (`sections.test.ts`). |
| UX-20 | General (theme, notifications, recordings folder, setup), Recording and About on the settings primitives; "Loading Preferences..." and the storage note are gone. |

Counts on `pnpm dev:mock` fixture data: **27 pages → 9**; visible switches
**77 → 29** (19 outside Advanced; Network page alone was 34).

New pages: General (Theme, Notifications, Recordings folder, Setup) ·
Recording (Permissions, Recording, Meeting detection, End automatically,
Tell people, Screen capture, Speakers, Speaker names from call apps) · Notes
(AI, About you, Spelling) · Templates · Calendar · Connections (Connected
apps, Auto-export, Export everything, Import, Watched folders) · Privacy
(Encryption, App lock, Retention, Sensitive information, Network, What left
this Mac) · Advanced (Notes after meetings, AI per task, Transcription,
Search, Voices, Shortcuts, Network services, Developer, Beta) · About.

## Follow-ups (PARITY_PLAN §7.2)

- Owner: `ai.autoSummary` defaults to `false` in Rust, so a new install does
  not write notes after meetings. The plan assumes it is on. The switch is
  in Advanced → Notes after meetings so nobody is stuck; flipping the
  default (a manual `Default` for `AiSettings`, then regenerating bindings)
  is a Rust change left out of this branch.
- UX-12 (other agent): the Beta page is now an Advanced section in
  `features/settings/settings-page.tsx`; delete that entry with the flag.
- UX-28 (other agent): the "Recording start notification" switch is gone;
  `lib/recordingNotification.tsx` still reads `recording.showRecordingNotice`
  (default on) until UX-28 removes the toast.
- UX-27 (other agent): Connections still shows "On the way" WIP integrations.
- UX-11: `ModelSettingsModal` (legacy, untyped) still backs "Other
  providers"; retire it behind the typed `llm` settings.
- UX-16: surface a failed search-model download outside Settings ("Search by
  meaning is off… Retry").
- IA: "Default template" on Notes is not built; templates stay on their page.
- Action items' page menu keeps its own "Remind me at 9:00" switch beside
  General → Notifications.
- `audio.usesHeadphones` is hidden; a user who turned it on keeps echo
  removal off until the field is cleared.
- Merge: `fix/engine-setting-source-of-truth` conflicts in
  `features/settings/settings-page.tsx` (take its `EngineSettings` inside
  `TranscriptionSection`), `features/settings/copy.ts` (keep its `engine`
  block; `groups` is deleted here) and `sections.test.ts` (take this
  branch's; mock `./components/engine-settings` instead of
  `@/components/TranscriptSettings`).
- ElevenLabs (CAP-14): add its card inside `TranscriptionSection`, or
  register a section `{ page: 'advanced', order: 25 }`; its service shows in
  Network services once it has a saved switch or traffic.

## Architecture (ARCHITECTURE.md)

- Frontend → settings: "Settings is nine host pages (`settingsPages`, from
  `features/settings/settings-page.tsx`) and sections features place on them
  (`settingsSections` with `page`, `order`, `aliases`) from their own
  `settings-page.ts(x)`. `/settings?section=<id>` accepts a page, a section
  or an alias; sections open scrolled into view. One Permissions card
  (Recording) owns every macOS permission row; features link to it with
  `PermissionLink`. `ui/more-options.tsx` holds rare options."

## CLAUDE.md

- "Rules that are easy to break", last bullet: settings pages → "settings
  sections, from a feature's own `settings-page.tsx` (export
  `settingsSections` with a host `page`, and add each id to
  `SettingsPageIds`); host pages are `features/settings/settings-page.tsx`".
- Merge workflow, REG-02 line: "settings pages" → "settings sections".

## Plan updates

- `docs/UX_SIMPLIFICATION.md`: UX-09, 10, 11, 13, 14, 15, 16, 17, 18, 19 and
  20 done (UX-11's Auto Summary moved, not deleted; see Follow-ups).
