# UX-nav-onboarding-copy · Sidebar, onboarding, toasts and copy (UX-21–27, UX-31–34)

Part of `docs/UX_SIMPLIFICATION.md`. Frontend only; no Rust, bindings or
settings changed.

## Progress log (PARITY_PLAN §7.1)

| Item | What shipped |
|---|---|
| UX-21 | Onboarding's Permissions step no longer shows Calendar (it has its own step). The AI step no longer shows the search-model row: `AutoStart` downloads it quietly while the step is open. |
| UX-22 | Setup asks only for the microphone and system audio (`UPFRONT_PERMISSIONS`, `upfront()` in `onboarding/permission-actions.ts`). Optional permissions are asked where they matter: `askPermissionOnce('accessibility')` (one toast with Allow, once per launch, only while macOS was never asked) when a detected Zoom, Teams or Meet recording starts (`shell/components/detection-recording-start.tsx`); `PermissionNudge kind="notifications"` (a quiet inline line with Allow) at the top of Home's Coming up. Screen Recording was already asked by screen capture's own card. |
| UX-23 | The AI step's model rows became one line, `TranscriptionStatus`: "Getting transcription ready… 42%" over a hairline bar, then "Transcription is ready". No codename or size. |
| UX-24 | Setup ends after Profile: `STEPS` has five steps; `try_it` stays in Rust's `OnboardingStep` as a retired step (old progress on it counts as finished: `isSetupStep`, the gate, `transition`). `TryDeino` (test recording and sample meeting, from `try-it-rows.tsx`) is exported from onboarding and shown in Home's Recent notes while there are no notes (latched for the visit so a test that creates the first meeting does not vanish). `try-it-step.tsx` deleted. |
| UX-25 | `nav.ts`: sections are `primary` (Home, Meetings, Action items, People, Folders tree), `more` (Insights, Recipes, Automations, under a collapsed "More" that opens while one of them shows) and `footer` (Settings). New `sidebar?: false` keeps Companies, Tags, Stats and Trash out of the sidebar but in ⌘K "Go to". No section labels. Companies is a tab beside People (`entities/components/entity-tabs.tsx`); Tags and Trash are in the Meetings header; Stats is a Home section (`StatsGlance`: the last 30 days, "See all stats"). |
| UX-26 | The sidebar's "Import recording" row is gone; Import is in the Meetings header (route passes `onImport`), ⌘K and drag-and-drop. |
| UX-27 | Hidden, code kept: ⌘K "Ask Deino" (removed from the actions list), ⌘J (`UNSHIPPED` in `shell/shortcuts.ts`; the toast is gone), `ChatPlaceholder` (no longer mounted), planned integrations (no longer listed; `PlannedList` kept), the person/company "Ask about" bar (`ENTITY_SLOTS.ask = null`), folder chat and folder Integrations placeholders (removed), planned automation steps, multi-meeting built-in recipes, and follow-ups' disabled "Send via Gmail or Outlook". |
| UX-31 | Silent success: legacy recording preferences and model settings, Ollama/built-in/Parakeet/Whisper download, cancel, switch and delete toasts, "Audio is back" / "Your Mac woke" (warnings still toast), profile saved, search rebuilt. |
| UX-32 | The policy is in `ui/toaster.tsx`'s doc comment. Swept every success/info call site: kept clipboard copies, files saved, off-screen work started (insights, automations, notes regenerating), undoable trash and edits, the folder-series offer and side-effect notices; removed confirmations of visible changes (tags, folders, people, companies, merges, recipes, templates, saved views, filing, tagging, restores, deletes, event links, voice profiles, follow-up dismiss, brief feedback, edit and correction undos, e-mail opened). A speaker rename toasts only when the voice was saved. |
| UX-33 | Sentence case and curly apostrophes: "Action items" (sidebar), "Don’t record", "Don’t show again", "didn’t", the quoted “Always ignore this app”. |
| UX-34 | "LLM usage", "LLM cost" → "AI usage", "AI cost" on Stats; no model codename in onboarding. |

Counts, before → after:

- Sidebar: 13 entries in 4 sections (2 labelled) + an "Import recording" row → 5 entries (Home, Meetings, Action items, People, Folders tree) + a collapsed "More" (3) + Settings; no labels.
- Onboarding: 6 steps → 5; Permissions step rows 6 → 2; AI step model rows 2 → 1 line.
- Toast calls (`frontend/src`, no tests or dev fixtures): success 69 → 25, info 12 → 4, plain `toast()` 15 → 12, warning 2 → 2 (98 → 43 non-error; one plain match is the toaster's doc comment); `toast.error` 100 → 100; `toast.custom` 1 → 1.

## Follow-ups (PARITY_PLAN §7.2)

- UX-28 owner: `lib/recordingNotification.tsx`'s "🔴 Recording Started" toast is still there (deleted with UX-13/UX-28).
- UX-16 owner: `search/copy.ts` Settings strings still say "index", "Keyword index", "Rebuild".
- UX-10 owner: `calendarPermission` and `PERMISSION_ICONS` (`onboarding/permission-icons.ts`) are ready for the Permissions card.
- Owner QA: the AI step still shows `ModelLicenceNotice`, which names the model for licence reasons; consider folding it under a "Licence" disclosure.
- DEV-05: a `deino://recipe` link still says "Recipes are not here yet" (`links/copy.ts`); Recipes has shipped, so it should open the Recipes page.
- Owner QA: `ComingSoonScreen` and `shellCopy.comingSoon*` are unused since no nav item sets `comingSoon`.
- Owner QA: recipes the user writes with `kind: 'multi'` (none today) would still show a WIP badge.

## Architecture (ARCHITECTURE.md)

- Frontend, shell: the nav registry has `primary`, `more` and `footer` sections and a `sidebar?: false` flag; ⌘K "Go to" lists every item.
- Frontend, onboarding: five steps; `TryDeino`, `PermissionNudge` and `askPermissionOnce` are exported for Home and the shell.

## CLAUDE.md

- Rules that are easy to break: "Toasts confirm only what the user cannot see (policy in `ui/toaster.tsx`)."
