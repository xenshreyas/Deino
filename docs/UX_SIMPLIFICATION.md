# UX simplification plan

The owner tested the built app and found it **overwhelming**. This plan sets out how to get it
to a calm, Granola-like feel: things show up only when they matter, defaults just work, advanced
options are tucked away, and there is no internal jargon. It is a plan only; no product code
changed. Paths are relative to `frontend/src/` unless they start with `crates/`. Line numbers
are from `main` at `f898bfba`.

## Where we are: the counts

| Surface | Today | Granola-like target |
|---|---|---|
| Settings pages | **27 pages** in **7 nav groups** (App, Capture, AI, Workspace, Privacy, Data, More) | **9 pages**, no group headings |
| Settings sections | about **46** titled sections (35 `SettingsSection` + 11 hand-built `<h3>/<h4>` in legacy panels) and 37 `SettingsCard`s | about 20 |
| Settings switches | **40 fixed switches**, plus **32** per-service switches on Network, plus 1 per calendar, watched folder and webhook (**72+** before the user adds anything) | about 15 visible; the rest under "More options" or in Advanced |
| Other settings controls | 13 per-task model rows (provider, model, Test each); 3 separate model pickers (AI models, Summary models, Transcription) | 1 AI picker; per-task models in Advanced |
| Meeting page | **4 tabs** (Notes, Transcript, Insights, Clips) + **4 right-panel tabs** (Chat, Comments, Participants, Details) + an **18-row** pipeline list + up to **5 blocks above the notes** | **2 tabs** (Notes, Transcript) + **2 panel tabs** (Chat, Details); status shown only on failure |
| Sidebar | **13 entries** in 4 sections (2 labelled) + "Import recording" row + record block | **5 entries** + Folders tree + Settings |
| Onboarding | **6 steps**; 6 permission rows (3 optional) + 2 model downloads | 5 steps; 2 required permissions; downloads in the background |
| Accessibility permission | asked on **5 surfaces** (onboarding, Detection, Speaker tags ×3 rows, Consent chat) | **1 row** in one Permissions card |
| Toasts | **84** success/info `toast` calls, **101** `toast.error` calls | success is silent unless it confirms something the user can't see |
| Pipeline stages | **18** registered stages, all listed; 5 have no readable label | 0 rows when everything worked |

There are 56 feature folders and 55 `copy.ts` files. Each feature added its own page, toggle or
status line, and nothing has pruned them since.

## Principles

1. **Status is silent unless it failed or needs action.** "Done" and "Skipped because not
   configured" are never shown. Running work is one quiet line, not a list.
2. **One permission = one row.** A macOS permission appears once, in one Permissions card. Each
   feature that needs it links there.
3. **One concept, one place, one name.** A setting lives on one page. A word means one thing.
   For example, "Enhance" can't mean both "write notes" and "re-transcribe".
4. **Defaults over choices.** If Deino can detect something (headphones, models, index state),
   it doesn't ask. Every switch must answer "who turns this off, and why?"
5. **Progressive disclosure.** Each page shows 3–5 everyday controls. Anything for one user in
   twenty goes under "More options" or in Settings → Advanced.
6. **Ship it or hide it.** No "WIP", "coming soon" or "On the way" in the product.
7. **Plain words, sentence case.** No stage IDs, model codenames, "re-pass", "embeddings",
   "index" or "fps" in default views.
8. **Toasts confirm only what isn't visible.** Saving a switch, a recovery that already showed
   in the pill, or a "success" the user can see on screen gets no toast.

## Prioritised changes

**Do first** marks the ten highest-impact, lowest-effort items. Effort: S is under a day, M is
1–3 days, L is a multi-card change.

### Meeting page

**UX-01 · Do first · Pipeline status is silent unless something failed** (S)
- *Now:* `features/meeting/components/meeting-header.tsx:67-68` mounts `PipelineStrip` under
  every meeting header. `features/pipeline/components/pipeline-strip.tsx:29-40` renders every
  stage, and `stage-item.tsx:33-39` shows its name, status and skip reason, for example "Tags
  Skipped There are no tags for Deino to apply." The reasons come from
  `crates/deino-pipeline/src/stages/*.rs`, for example `final_transcription.rs:29` and
  `auto_folder.rs:21`.
- *Change:* Show only `failed | cancelled` stages, one plain-words row each with Retry ("Notes
  couldn't be written. Retry"). While any stage is `running` or `pending`, show one muted line
  ("Writing notes…"). Never show `succeeded` or `skipped`.
- *Why:* This is the owner's first complaint, and none of these rows are actionable. *Risk:*
  Low. Keep the full list in UX-05's "Processing details" for debugging.

**UX-02 · Fix the stage labels** (S)
- *Now:* `features/pipeline/copy.ts:24-43` has no label for `finalize_audio`, `screen_video`,
  `index`, `update_entities` or `automations`. The `stageLabel` fallback
  `stage.replaceAll('_', ' ')` produces "finalize audio" and "update entities" next to
  "Speakers".
- *Change:* Add a sentence-case label for all 18 stages ("Audio", "Screen video", "Search",
  "People and companies", "Automations"). Make the map a total `Record<StageId, string>` so a
  new stage fails to compile without one.
- *Why:* Failures from UX-01 still need readable names. *Risk:* None.

**UX-03 · Two tabs: Notes and Transcript** (M)
- *Now:* `features/meeting/slots.ts:36-41` registers 4 tabs. Insights is usually empty ("No
  insights on this meeting yet", `features/insights/copy.ts:78`), and so is Clips ("No clips
  yet", `features/clips/copy.ts:3`).
- *Change:* Drop both tabs. Insight answers become a section at the end of Notes, only when
  they exist; "Run an insight" moves to the ⋯ menu. Clips are listed in Details (UX-04).
  Map `?tab=insights` to Notes (`features/insights/components/insight-table.tsx:199` links
  there).
- *Why:* Two of the four tabs are usually empty, and Granola has one surface with a
  transcript toggle. *Risk:* The `MeetingTabId` schema and the ⌘3/⌘4 shortcuts change.

**UX-04 · Right panel: Chat and Details** (M)
- *Now:* `features/meeting/slots.ts:44-49` registers 4 panel tabs: Chat, Comments,
  Participants, Details. Details (`slots/details-panel.tsx:43-50`) shows created and edited
  dates, links and vocabulary corrections.
- *Change:* Details becomes Participants, then Comments and clips (only when there are any),
  the calendar event and related meetings, then "Corrections" collapsed. Drop the Created and
  Last edited facts, since the header shows the date.
- *Why:* Comments and Participants are short lists that don't need tabs. *Risk:* The comment
  layer's panel target changes.

**UX-05 · One ⋯ menu for rare meeting actions** (S)
- *Now:* The ⋯ menu (`meeting-header.tsx:52-53`) holds only "Automations". The Transcript tab
  has the legacy `TranscriptButtonGroup`
  (`components/MeetingDetails/TranscriptButtonGroup.tsx:37-62`): "Recording" ("Open Recording
  Folder") and "Enhance", which actually re-transcribes.
- *Change:* The ⋯ menu holds Re-transcribe…, Show recording in Finder, Run automation ›, Run
  insight ›, Processing details (the read-only UX-01 list) and Move to trash. Remove the
  button group.
- *Why:* This frees the transcript header and removes a second "Enhance" (the title bar's
  `EnhanceButton` writes notes). *Risk:* Low.

**UX-06 · Do first · The template line shows no reasoning** (S)
- *Now:* `features/templates/components/meeting-template-picker.tsx:53` always prints the reason
  next to the template name, for example "Matched rule: external discovery call" or
  "Suggested by AI: …" (`:100-109`).
- *Change:* Show only the template name as a quiet menu button. Move the reason into the
  menu's header or a tooltip.
- *Why:* The reason is internal rule-matching detail on every meeting. *Risk:* None.

**UX-07 · Fewer blocks above the notes** (M)
- *Now:* `features/meeting/slots/notes-slot.tsx:73-81,105-109` stacks the brief, the follow-up
  banner, the Images strip, the template line and action items before the notes start.
- *Change:* Show the brief only before or during the meeting. The follow-up banner becomes a
  "Follow-up email ready" header chip, and Images moves after the notes. Action items stay.
  The template menu joins the Enhanced/My notes toolbar.
- *Why:* The notes are the product, and today they start a screen down. *Risk:* The
  `notes-slot` and `follow-up-banner` tests change.

**UX-08 · Fewer chips in the header** (S)
- *Now:* `meeting-header.tsx:55-64` shows a date chip, calendar chips, attendees, folders,
  tags and Resume. The "Add …" ghosts already hide until hover.
- *Change:* Keep date, attendees and folder. Show tags only when set, and Resume only for a
  meeting that ended less than 2 hours ago.
- *Why:* A small win, but the header is the first thing seen. *Risk:* None.

### Settings

**UX-09 · Do first · One Accessibility row, not three** (S)
- *Now:* Settings → "Speaker tags" (`features/speaker-tags/settings-page.ts:13`, section titled
  "Meeting apps" at `copy.ts:6`). `app-tag-row.tsx:45-71` renders Zoom, Teams and Google Meet,
  each with its own "Needs permission" badge and "Open System Settings" button, for one macOS
  Accessibility permission.
- *Change:* One row, "Accessibility: needed to read speaker names from Zoom, Teams and Meet",
  with one button when it's missing. List the apps as plain text below it, and mark an app
  only when it is `unsupported`.
- *Why:* This is the owner's second complaint (principle 2). *Risk:* None.

**UX-10 · One Permissions card** (M)
- *Now:* Permission UI is built five times: onboarding `permission-row.tsx`, Detection's
  `SupportNote` (`features/settings/components/detection-settings.tsx:180-189`), Speaker tags
  (UX-09), Consent's `PermissionRow` (`features/consent/components/chat-message-settings.tsx:71-81`)
  and Screen capture's `permission-card.tsx`.
- *Change:* Add one "Permissions" card at the top of Settings → Recording, reusing
  onboarding's `PermissionRow`: microphone, system audio, Accessibility, Screen Recording,
  Calendar. Granted rows show only "Allowed". Features show a one-line link ("Needs
  Accessibility") while a permission is missing.
- *Why:* One permission = one row, everywhere. *Risk:* Frontend wiring only;
  `deino-core::permissions` already exists.

**UX-11 · Merge "AI models" and "Summary models" into one "AI" section** (M)
- *Now:* There are two pages for the same job:
  - "AI models" (`features/llm/settings-page.ts:13`): providers plus a 13-row "Models per task"
    table with Test buttons. Its copy says "Add or replace them under Summary models"
    (`features/llm/copy.ts:7`).
  - "Summary models" (`features/settings/settings-page.tsx:147-153`): the inherited "Auto
    Summary" switch ("Auto Generating summary after meeting completion(Stopping)",
    `components/SummaryModelSettings.tsx:32-33`) and the 1,234-line `ModelSettingsModal` (Max
    Tokens, Temperature, Top P at `:851-876`).
- *Change:* One section with onboarding's three choices (Claude, OpenAI, On this Mac), a key
  field and one "Check". Groq, OpenRouter, Ollama and custom endpoints go under "Other
  providers", and the per-task table moves to Advanced. Delete Auto Summary: notes are
  always written.
- *Why:* This is the most confusing pair in the app; Granola has no model picker. *Risk:*
  `ModelSettingsModal` is legacy and untyped. Retire it behind the typed `llm` settings.

**UX-12 · Delete the Beta page** (S)
- *Now:* `features/settings/settings-page.tsx:174-180` registers a whole page for one switch,
  `importAndRetranscribe` (`types/betaFeatures.ts:32`, on by default), which gates
  re-transcribe.
- *Change:* Remove the page and the flag. Re-transcribe lives in the ⋯ menu (UX-05).
- *Why:* A page with one switch that defaults on. *Risk:* None.

**UX-13 · Remove switches that duplicate or defeat defaults** (S)
- *Now:* `components/RecordingSettings.tsx` has four controls to remove:
  - "Save Audio Recordings" (`:72-84`); turning it off breaks re-pass, clips and the player.
  - "Recording Start Notification" (`:120-132`), a duplicate of Consent's "Reminder when
    recording starts" (`features/consent/copy.ts:17`).
  - "I use headphones" (`:134-148`), whose own copy says "Deino detects headphones on its own".
  - A "File Format: MP4 files" box (`:103-106`).
- *Change:* Delete all four. Recordings are always saved, and retention already deletes them.
- *Why:* Principle 4. *Risk:* Medium. Check the Rust readers of `audio.usesHeadphones` and
  `recording.autoSave` first; keep the fields and stop showing them.

**UX-14 · Network page shows only what's in use** (S)
- *Now:* `features/network/components/service-list.tsx:47` renders a switch for each of 32
  services (`features/network/copy.ts:35-67`), including Trello, Zoho CRM and Fathom import.
  Most say "Set up in Settings first".
- *Change:* List only configured or used services. The full allow-list goes behind "Show all
  services" in Advanced. Offline mode stays as the one switch in Privacy.
- *Why:* 32 switches for things the user never set up. *Risk:* None.

**UX-15 · Collapse capture options behind "More options"** (S)
- *Now:* Several features show every option at once:
  - Screen capture (`features/screen/components/capture-card.tsx:63-125`): 4 switches and an
    fps "Frame rate" picker (`:74-80`).
  - Auto-export (`features/export/components/auto-export-settings.tsx:98-151`): 4 switches,
    a file-name template, subfolder, sections, "Obsidian" and "sidecar".
  - Detection (`detection-settings.tsx:58-75`): a countdown-seconds select.
- *Change:* Each feature shows one primary switch. The rest sits under a closed "More
  options" (`ui/accordion.tsx`).
- *Why:* Principle 5. *Risk:* None.

**UX-16 · Search index maintenance moves to Advanced** (S)
- *Now:* "Search index" (`features/search/settings-page.ts:13`, AI group) has a "Search model"
  download, "Meetings indexed by meaning", "Index past meetings" and "Rebuild keyword index"
  (`features/search/copy.ts:34-56`).
- *Change:* Download the model and backfill automatically after onboarding, and move the
  page to Advanced as "Search". Surface only a failure ("Search by meaning is off: the model
  couldn't download. Retry").
- *Why:* This is maintenance, not a preference. *Risk:* The download is about 100 MB, so
  respect offline mode.

**UX-17 · One place for notifications** (M)
- *Now:* Notification switches are in four places: General ("Notifications… of start and end
  of meeting", `components/PreferenceSettings.tsx:75-78`), Calendar reminders
  (`features/calendar/components/rules-settings.tsx:110-150`), pre-meeting briefs
  (`features/briefs/components/brief-settings.tsx:44`) and the Action items page menu
  ("Remind me at 9:00", `features/action-items/copy.ts:130`).
- *Change:* General → "Notifications" lists each kind as one switch.
- *Why:* One concept, one place. *Risk:* None; the settings keys stay where they are.

**UX-18 · Rename and merge overlapping pages** (S)
- *Now:* Several pages overlap in name or job:
  - Settings "People" (voice profiles, `features/speakers/settings-page.ts:13`) collides with
    the sidebar's People.
  - "Import" (`features/importers/settings-page.ts:13`), "Watched folders"
    (`features/import/settings-page.ts:13`) and the sidebar's "Import recording" are three
    imports.
  - Profile's "Context and glossary" (`features/profile/copy.ts:20`) overlaps Vocabulary's
    "Words and names" (`features/vocabulary/copy.ts:9`).
- *Change:* Voice profiles become "Voices" in Advanced. Make one "Import" page (from another
  app, watched folders). The Profile glossary hint links to Words and names.
- *Why:* Principle 3. *Risk:* `SettingsPageIds` change, so alias the old ids in
  `settingsSearchSchema`.

**UX-19 · Consolidate to 9 pages (target IA below)** (L)
- *Now:* 27 pages: 17 come from feature `settings-page.ts(x)` files and 10 from
  `features/settings/settings-page.tsx`, in 7 groups (`features/settings/sections.ts:20-28`).
- *Change:* Implement the target IA. Keep REG-02 self-registration: features register
  *sections* into a smaller set of host pages (add a `page` field next to `group`).
- *Why:* This is the biggest structural cause of "overwhelming". *Risk:* Medium. It touches
  every `settings-page.ts`, `scripts/check-feature-isolation.sh` and deep links
  (`/settings?section=…`, `deino://`), so keep an id-alias map.

**UX-20 · Bring the legacy panels to the design system** (M)
- *Now:* General, Recording, Summary models and About use `text-lg` headings, shadowed cards,
  Title Case and a "Loading Preferences..." text (`components/PreferenceSettings.tsx:66`).
  General's description promises "how Deino starts" (`features/settings/settings-page.tsx:107`),
  which the page doesn't have. It also shows a "Note: Database and models are stored
  together…" box beside commented-out rows (`PreferenceSettings.tsx:89-140`).
- *Change:* Rebuild these panels with `SettingsSection`/`SettingsRow`, skeletons and sentence
  case, and drop the note.
- *Why:* The mixed styles make Settings feel like two apps. *Risk:* Low.

### Onboarding

**UX-21 · Do first · Drop the duplicate rows** (S)
- *Now:* The Permissions step has a Calendar row
  (`features/onboarding/components/steps/permissions-step.tsx:44,77-90`), and step 4 asks for
  Calendar again (`calendar-step.tsx:19`). The AI step shows a "Search" model download next to
  Transcription (`ai-setup-step.tsx:47`).
- *Change:* Remove Calendar from the Permissions step. Download the search model silently
  after setup (UX-16).
- *Why:* It asks twice, and shows a download the user can't decline. *Risk:* None.

**UX-22 · Ask optional permissions in context** (M)
- *Now:* Step 2 lists 6 rows (`features/onboarding/copy.ts:49-65`), including Notifications,
  Accessibility and Screen Recording (two marked "Optional").
- *Change:* Onboarding asks only for the microphone and system audio. Ask for Accessibility on
  the first detected Zoom, Teams or Meet call, for Screen Recording when screen capture is
  turned on, and for Notifications before the first reminder.
- *Why:* Granola asks for two things up front. *Risk:* Each feature needs a contextual prompt;
  reuse UX-10.

**UX-23 · Plain words in the AI step** (S)
- *Now:* "Parakeet, English speech to text." (`features/onboarding/copy.ts:75`), and two
  model rows with percentages.
- *Change:* One line, "Getting transcription ready… 42%", with no codename.
- *Why:* Principle 7. *Risk:* None.

**UX-24 · Fold "Try it" into Home** (S)
- *Now:* Step 6 (`features/onboarding/copy.ts:108-130`) offers a 20-second test and a sample
  meeting.
- *Change:* Setup ends after Profile. Home's empty state offers "Record a test" and "Open a
  sample meeting".
- *Why:* One fewer step, and the empty state is where a new user looks. *Risk:* Low.

### Sidebar and navigation

**UX-25 · Do first · Trim the sidebar to 5 entries** (S)
- *Now:* `features/shell/nav.ts:63-163` has 13 entries under "Workspace" and "Intelligence"
  labels. Trash appears twice: in the sidebar (`nav.ts:123-130`) and in the Meetings header
  (`features/meetings-list/components/meetings-list-screen.tsx:35`).
- *Change:*
  - Visible entries: Home, Meetings, Action items and People, then the Folders tree and
    Settings. Drop the two section labels.
  - Companies becomes a tab on People.
  - Tags and Trash live in the Meetings header, and Stats moves to Home.
  - Insights, Recipes and Automations go under one collapsed "More".
  - Add `sidebar?: false` to `NavItem` so ⌘K "Go to" keeps every page.
- *Why:* Granola's sidebar is notes plus folders. *Risk:* Low.

**UX-26 · Move "Import recording" out of the sidebar footer** (S)
- *Now:* `features/shell/sidebar/sidebar.tsx:18-33,39` puts an always-visible import row
  above Settings.
- *Change:* Keep import in ⌘K, drag-and-drop (`AudioDropTarget`) and the Meetings header.
- *Why:* A rare action in prime space. *Risk:* None.

**UX-27 · Do first · Hide unfinished features** (S)
- *Now:* Four unfinished features are visible:
  - ⌘K lists "Ask Deino" with a "WIP" badge (`features/shell/command-palette/actions-provider.tsx:56-63`).
  - ⌘J toasts "Ask Deino is coming soon." (`features/shell/hooks/use-global-shortcuts.ts:26-27`).
  - `ChatPlaceholder` is still mounted (`features/shell/app-shell.tsx:51`).
  - Integrations shows an "On the way" list of WIP providers
    (`features/integrations/components/integrations-settings.tsx:89-91`, `copy.ts:17-22`).
- *Change:* Remove all four until the feature ships.
- *Why:* Principle 6. *Risk:* None.

### Recording UI

**UX-28 · One reminder at most when recording starts** (S)
- *Now:* Starting a recording can show three versions of the same message:
  - the legacy toast "🔴 Recording Started… Inform all participants…" with a checkbox
    (`lib/recordingNotification.tsx:22`, called from
    `features/recording/components/recording-sync.tsx:78`);
  - Consent's "Let people know" card under the pill (`record-panel.tsx:171`);
  - the screen-capture reminder card (`record-panel.tsx:172`).
- *Change:* Delete the legacy toast and its setting (with UX-13). Merge the screen reminder
  into the consent card when screen capture is on.
- *Why:* Three versions of one message, one with an emoji. *Risk:* None.

**UX-29 · Fewer pill controls** (S)
- *Now:* The hover reveal (`features/recording/components/record-panel/pill-controls.tsx:56-88`)
  shows Pause, Stop, Microphone, Screen, a system-audio toggle ("Not recording the call's audio
  — click to record it") and Open notes.
- *Change:* Keep Pause, Stop and Open notes. Move Microphone, Screen and System audio into a
  "⋯" menu.
- *Why:* Mid-meeting, the user needs stop and notes; device switches are rare. *Risk:* Low.

**UX-30 · Quieter detection prompt** (S)
- *Now:* `features/detection/components/meeting-prompt.tsx:73-95` shows "Always ignore this
  app", Ignore and Take notes.
- *Change:* Show two buttons, and move "Always ignore this app" into a menu on Ignore.
- *Why:* A permanent choice shouldn't sit beside the everyday ones. *Risk:* None.

### Notifications and toasts

**UX-31 · Do first · Silent success for settings and recoveries** (S)
- *Now:* Success toasts that confirm nothing new:
  - "Preference saved" on every legacy switch (`components/RecordingSettings.tsx:81,130,145`);
  - "Model settings saved successfully" (`components/SummaryModelSettings.tsx:18`);
  - "Model … downloaded!" (`contexts/OllamaDownloadContext.tsx:52`);
  - "Audio is back" and "Your Mac woke; recording continues"
    (`features/recording/components/recording-sync.tsx:38-43`).
- *Change:* Settings save silently, with an inline error on failure (as the typed pages do).
  Recoveries clear the pill's warning, and downloads finish quietly.
- *Why:* Principle 8. *Risk:* None.

**UX-32 · A toast policy** (S)
- *Now:* 84 success/info toasts, many confirming visible changes ("Tag created", "Folder
  created", "Profile saved", "Rebuilt: N items indexed").
- *Change:* Write the rule into `ui/toaster.tsx`'s doc comment: toast only clipboard copies,
  off-screen background work and undoable destructive actions. Sweep the rest in passing.
- *Why:* This keeps it from creeping back. *Risk:* None.

### Copy

**UX-33 · Do first · Sentence case and one name per thing** (S)
- *Now:* Three kinds of inconsistency:
  - Casing: "Action Items" in the sidebar (`features/shell/nav.ts:75`), but "Action items"
    elsewhere. Legacy Title Case in "Save Audio Recordings", "Recording Start Notification",
    "Default Audio Devices" (`components/RecordingSettings.tsx:74,123,153`), "Data Storage
    Locations", "Meeting Recordings" (`components/PreferenceSettings.tsx:84,122`) and
    "Transcript Model" (`components/TranscriptSettings.tsx:43`).
  - Apostrophes: "Don't show again" (`features/screen/copy.ts:47`) versus "Don’t show again"
    (`features/consent/copy.ts:10`).
  - Names: two "Enhance" buttons (UX-05).
- *Change:* Sentence case, curly apostrophes, and one verb per action.
- *Why:* Inconsistency reads as unfinished. *Risk:* None; text-matching tests change.

**UX-34 · Remove jargon from default views** (S)
- *Now:* Default views use internal terms:
  - "High-accuracy re-pass" (`features/settings/copy.ts:32`)
  - "Parakeet Unified EN (Recommended, live)" and "Parakeet TDT v3 (ONNX)"
    (`components/TranscriptSettings.tsx:54-57`)
  - "Models per task", embeddings and "dimension vectors" (`features/llm/copy.ts:12-27`)
  - "Keyword index" and "Rebuild" (`features/search/copy.ts:52-55`)
  - "deino:// links" (`features/hotkeys/settings-page.ts:14`)
  - "1 fps" (`features/screen/copy.ts:31`)
- *Change:* Say "Re-transcribe after the meeting for better accuracy". The transcription
  engine ("Standard (recommended)" or "Alternative"), links and per-task models go to
  Advanced.
- *Why:* Principle 7. *Risk:* None.

## Target information architecture

### Settings: 9 pages, no group headings

| Page | Contents (in order) | Comes from |
|---|---|---|
| **General** | Theme · Notifications (one switch per kind) · Recordings folder · Run setup again | General, Appearance, Setup, calendar reminders, briefs, action-item reminder |
| **Recording** | Permissions card · Microphone · Detect meetings · End automatically · Remind me to tell people · Screen capture · Recognise speakers by voice (details under More options) | Recording, Meeting detection, Consent, Screen capture, Speaker tags, People (switch) |
| **Notes** | AI provider and key · About you (profile, style, context) · Words and names · Default template | AI models + Summary models (basics), Profile, Vocabulary |
| **Templates** | Template library and editor | Templates |
| **Calendar** | Calendars and connections · Auto-record rules · Pre-meeting briefs | Calendar |
| **Connections** | Connected apps · Auto-export · Import (other apps, watched folders) · Export everything | Integrations, Export, Import, Watched folders |
| **Privacy** | Lock and encryption · Delete after N days · Hide sensitive information · Offline mode · What left this Mac | Privacy & security, Network (redaction, offline) |
| **Advanced** | Models per task · Transcription engine, live/after · Search · Voices · Shortcuts and links · Network services and log · Developer (CLI, API, webhooks) | AI models (table), Transcription, Search index, People, Shortcuts, Network, Developer |
| **About** | Version, privacy points, licences | About |

The pages named in the "Comes from" column are deleted (Beta goes entirely). That takes
Settings from 27 pages to 9, and from 72+ switches to about 15 visible ones.

### Meeting page

- **Title bar:** ‹ Meetings · Enhance · Share · panel toggle (unchanged).
- **Header:** title · ⋯ menu (UX-05) · date, attendees and folder chips (tags when set) · one
  status line only while processing or on failure.
- **Tabs:** **Notes** (action items; the notes with the Enhanced/My notes switch and template
  menu; insight answers and images when present) · **Transcript**.
- **Right panel:** **Chat** · **Details** (participants; comments and clips when present;
  event and related meetings; corrections collapsed). **Player:** unchanged.

### Sidebar

New note · Search (⌘K) / **Home · Meetings · Action items · People** / Folders tree / More ▸
(collapsed: Insights, Recipes, Automations) / footer: **Settings** · record block. Every page
stays reachable from ⌘K "Go to".

## Suggested order

1. The ten **Do first** items as small PRs (UX-01, 06, 09, 21, 25, 27, 31, 33, plus 12 + 05
   and 28 + 13 as pairs). All are S and frontend-only; UX-13 keeps the settings fields.
2. UX-10 and UX-11 (the Permissions card and the single AI section), which unblock the
   consolidation.
3. UX-19, folding in UX-14 to UX-18 and UX-20.
4. UX-03, 04 and 07 (meeting page structure), then onboarding UX-22 to UX-24.

Each change follows `docs/PARITY_PLAN.md` §3, §3A and §3B, and writes a
`docs/changes/<ID>.md` rather than editing shared docs.

## Out of scope and open questions for the owner

1. **Auto Summary:** does anyone need notes *not* written after a meeting? UX-11 assumes no.
2. **Saving audio:** UX-13 assumes audio is always kept until retention deletes it. Do you
   want a "transcript only, never keep audio" option in Privacy instead?
3. **Home and Meetings** both list meetings, and Granola has one list. This plan keeps both:
   Home as "Coming up + recent", Meetings as the archive. Merge them?
4. **Insights, Recipes and Automations:** are these daily tools for you? The "More" group
   assumes they are not. If you use one daily, promote it.
5. **Consent chat message:** posting into Zoom, Teams or Meet chat is powerful but
   surprising. Keep it under More options, or move it to Advanced?
6. **Stats / Year in review:** keep it as a Home section, or hide it until December?
7. **Not covered here:**
   - Visual polish such as spacing, type and colour (the design system covers it).
   - The tray menu (Rust, not audited).
   - Rust skip-reason wording, which UX-01 hides anyway.
   - Pipeline behaviour: stages keep running; only their display changes.
