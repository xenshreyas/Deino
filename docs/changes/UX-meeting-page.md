# UX-meeting-page · Meeting page and recording UI (UX-01–08, UX-12, UX-28–30)

Part of `docs/UX_SIMPLIFICATION.md`. Frontend only: no pipeline, Rust or
settings-schema change. Hidden settings keep their fields
(`beta.importAndRetranscribe`, `recording.showRecordingNotice`).

## Progress log (PARITY_PLAN §7.1)

| Item | What shipped |
|---|---|
| UX-01 | `PipelineStrip` is gone. The header's `PipelineStatus` (`features/pipeline`) says nothing when every stage worked or was skipped, one quiet line while work runs ("Writing notes…"), and one plain-words row per failed or cancelled stage with its reason and Retry ("Notes couldn’t be written."). Every stage, done or skipped, is in `ProcessingDetailsDialog`, opened from the ⋯ menu. |
| UX-02 | `features/pipeline/copy.ts`: `KNOWN_STAGES` (all 18 stages, in `deino_pipeline::registry` order) and a total `Record<KnownStage, …>` with a sentence-case label, a "running" line and a "failed" line per stage, so a stage added to the tuple without words fails to compile. An unknown stage reads "Processing", never its ID. |
| UX-03 | Two tabs, Notes and Transcript (`slots.ts`, `meetingTabSchema`); ⌘1 and ⌘2 only. `?tab=insights` and `?tab=clips` still parse and open Notes (`tabParamSchema` in `features/meeting/search.ts`); the Insights page's meeting links write `tab: 'notes'`. Insight answers are `MeetingInsights`, a section after the notes shown only when an insight ran there. |
| UX-04 | Two panel sections, Chat and Details. Details (`slots/details-panel.tsx`): Participants, then Comments and Clips only when there are any (`CommentsPanel` and the new `MeetingClips` render nothing otherwise), the calendar event (with its link/change chip, moved from the header) and related meetings, then Corrections collapsed in an accordion. Created and Last edited are gone. New shared primitive `ui/panel-section.tsx`. |
| UX-05 | The meeting's ⋯ menu (`components/meeting-menu.tsx`): Re-transcribe…, Show recording in Finder, Run automation ›, Run insight ›, Processing details, Move to trash (with Undo, then back to Meetings). The legacy `TranscriptButtonGroup` (Recording, and an "Enhance" that re-transcribed) is deleted; the legacy session now publishes only the recording folder (`legacy/panels-context.tsx`). Re-transcribe actions live in `components/recording-menu-items.tsx`, one item each, so "Re-transcribe with ElevenLabs" slots in beside "Re-transcribe…". Both recording items are disabled until the folder is known, and for meetings with no recording on this Mac. |
| UX-06 | The template is a quiet `MeetingTemplateMenu` button with only its name; the reason ("Matched rule: …") is the menu's header. |
| UX-07 | The brief shows only while the meeting records or before its calendar event ends (`hooks/use-is-live.ts`). The follow-up banner is a "Follow-up email ready" header chip (`FollowUpChip`: View draft, Dismiss for good). The template menu sits in the notes' toolbar beside Enhanced / My notes; after a switch, `TemplateSwitchedPrompt` offers to regenerate. Insight answers and Images follow the notes inside their scroll (a new `footer` prop on `EnhancedNotes`, `NotesDocument`, `Notepad`, `NoteSurface`). Action items stay. |
| UX-08 | Header chips: date, attendees, folders, tags (their "Add" ghosts still appear on hover), the follow-up chip. The calendar chips moved to Details. Resume shows only for a meeting that ended under two hours ago (`canResume`; the end is the earliest pipeline stage update, else when the meeting was saved). |
| UX-12 | The Beta settings page, `components/BetaSettings.tsx`, `types/betaFeatures.ts` and `ConfigContext`'s `betaFeatures`/`toggleBetaFeature` are deleted; re-transcribe is always in the ⋯ menu. The `beta` section stays in the settings schema, unread. |
| UX-28 | `lib/recordingNotification.tsx` (the "🔴 Recording Started" toast) and its call in `RecordingSync` are deleted, and the "Recording Start Notification" switch is gone from Settings → Recording. The record panel shows at most one reminder (`record-panel/reminder.tsx`): Consent's card, whose body mentions the shared screen while it is captured (and whose answers then answer the screen reminder too), or screen capture's own card when the consent reminder is off. Consent and screen expose their state as hooks (`useConsentReminder`, `useScreenReminder`); `ScreenReminder` and `showCaptureMenu` are gone. |
| UX-29 | The pill shows Pause, Stop, Open notes and ⋯. ⋯ is a native menu (`showPillMenu` in `panel-window.ts`): Microphone ›, Screen to capture › (`captureMenuItems` from `features/screen`), and a "Record the call’s audio" check item. |
| UX-30 | The detection prompt has two buttons, Ignore (or Don’t record) and Take notes; "Always ignore this app" is in a menu on Ignore's chevron, opening upward inside the prompt window. |
| UX-33/34 | In the files touched: sentence case and curly apostrophes ("Re-transcribe this meeting", "Don’t record", "Don’t show again"), one name per action (the re-transcribe item is "Re-transcribe…", never "Enhance"; the menu says "Run automation" and "Run insight"; the vocabulary stage is "Words and names" as in Settings), "Corrections" for the vocabulary list, no stage IDs anywhere. |

Tests: `pipeline-status.test.tsx` (silence, running line, failure row and retry, Processing details, stage words for all 18 stages), `slots.test.ts`, `-routes.test.ts` (old tab links), `resume-recording.test.ts`, `reminder.test.tsx`, `follow-up-chip.test.tsx`, `meeting-template-picker.test.tsx`, `meeting-insights.test.tsx`, `meeting-links-panel.test.tsx` (was the header chips test), `meeting-corrections.test.tsx`, `pill.test.tsx`, `detection-prompt.test.tsx`, `participants-panel.test.tsx`; e2e specs for insights, automations, event links, follow-ups, templates, speakers, clips and comments updated. The full Playwright suite passes over `dev:mock`.

## Follow-ups (PARITY_PLAN §7.2)

- ElevenLabs agent: add "Re-transcribe with ElevenLabs" as another item in `features/meeting/components/recording-menu-items.tsx`, next to "Re-transcribe…".
- CAP-08/CAP-11: port re-transcription to a typed command; then `features/meeting/legacy/` (the session that only publishes the recording folder) and the legacy `RetranscribeDialog` can go, and Re-transcribe and Show in Finder work without the untyped metadata. In `dev:mock` both items are disabled because the legacy metadata has no `folder_path`.
- Settings agent (UX-13): `recording.showRecordingNotice` and `beta.importAndRetranscribe` are now unread by the UI; drop or deprecate the fields when the schema next changes. `features/settings/legacy-local-storage.ts` still migrates the old `betaFeatures` key, harmlessly.
- Settings agent: `detectionCopy.settings` still says "Don't record" with a straight apostrophe (left to avoid a conflict with the detection settings rework).
- Toasts agent (UX-31/32): `RetranscribeDialog` still toasts "Re-transcription finished" and "Re-transcription cancelled"; the brief's "thanks" and participant toasts are unchanged.
- Owner QA: the header's invisible "Add to folder"/"Add tag" ghosts still take room at rest, so a long chip row can wrap (e.g. the follow-up chip on a second line). Consider revealing the ghosts in a single "+" chip.
- Owner QA: "Run insight" lists insights already run here too (as "run again"); the Insights page is where new ones are created.
- Owner QA: the pill's ⋯ native menu and the detection prompt's Ignore menu need a check in the real app (native menus do nothing in `dev:mock`).

## Architecture (ARCHITECTURE.md)

- Meeting page: two tabs (Notes, Transcript) and two panel sections (Chat, Details); the header carries the ⋯ menu (`features/meeting/components/meeting-menu.tsx`) and the quiet `PipelineStatus`; Processing details is a dialog. Old `?tab=insights|clips` links open Notes.
- Record panel: the pill's device switches are a native ⋯ menu; one reminder card under the pill.

## CLAUDE.md

- Meeting tabs and panels are still added through `features/meeting/slots.ts`, but UX-03/UX-04 cap them at two each: add a section to Notes or Details before adding a tab.

## Plan updates

- `docs/UX_SIMPLIFICATION.md`: UX-01–UX-08, UX-12, UX-28–UX-30 done; UX-33/UX-34 applied to the meeting page and recording UI.
