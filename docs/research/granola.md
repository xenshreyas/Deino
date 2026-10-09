# Granola (granola.ai): Complete Feature Inventory as of 2026-10-08

Research date: 2026-10-08. The primary source is the official help center at docs.granola.ai. All ~110 pages listed in `https://docs.granola.ai/llms.txt` were downloaded as raw markdown on 2026-10-08. The help center pages carry no dates, so treat them as describing the **current** state. Other sources are the official changelog (granola.ai/updates), blog posts (dated), the pricing, integrations, enterprise and security pages, the App Store and Play Store listings, the API reference and changelog, HN (Algolia API), and third-party reviews and comparisons. Transcription-model quality and multilingual details are mostly left out, as requested.

Conventions:
- **[DOC]** = docs.granola.ai help center, fetched 2026-10-08 (current state).
- **[CL]** = official changelog or blog post, dated.
- **[3P]** = third-party source. Lower confidence; several are competitor-written.
- Plan names: **Basic** (free), **Business** ($14/user/mo), **Enterprise** (from $35/user/mo).

---

## 0. TL;DR: What Granola is in Oct 2026

- It is a **bot-free AI notepad**. A desktop app (Electron, macOS and Windows) captures **mic plus system audio** locally and streams it to cloud transcription (Deepgram or AssemblyAI). No bot joins the call, and **no audio is stored**.
- The core UX is a **plain notepad**. The user types rough notes during the meeting. When the meeting ends, Granola "enhances" them into structured notes: the user's own text stays **black** and AI-added text is **grey**. Each AI bullet has a **magnifying glass** that shows where it came from in the transcript.
- Around this core it has grown into a "company context" platform:
  - **Granola Chat**, an agent rebuilt in Apr 2026. It has inline citations, "coverage notes", Recipes (slash-command saved prompts), model picker, actions (send Gmail, post to Slack, create Google Calendar events) and web research.
  - **Spaces and folders**: My notes, Team space, custom spaces, nested folders, and auto-add for recurring meetings.
  - **People and Companies** pages.
  - **Briefs**: overnight pre-meeting prep.
  - **Suggested follow-up emails** sent via Gmail.
  - **Speaker tags** for Meet, Zoom and Teams via Accessibility APIs or a Chrome extension.
  - **Transparency features**: an automated chat message, plus a virtual-camera **watermark**.
  - **MCP server** at `https://mcp.granola.ai/mcp`.
  - **Public REST API**, **webhooks**, **Audit API** and **Legal Holds API**.
  - CRM integrations: HubSpot, Attio, Affinity, plus Salesforce and others via Zapier.
  - **Mobile apps**: iOS (with phone calls through a built-in dialer), Android (Jul 2026) and Apple Watch (Jul 2026).
- Funding: $20M Series A (Oct 2024), $43M at a $250M valuation (May 2025), and a $125M Series C at a $1.5B valuation led by Index (25 Mar 2026). The team is based in Shoreditch, London.
- Headwinds:
  - A **class action** (*Chamberlain v. Granola*, N.D. Cal., filed 30 Jul 2026) alleges wiretapping and training on data without consent.
  - A Verge story (Apr 2026) reported that notes were link-shareable by default.
  - A Strix one-click account takeover was found and fixed in Jun–Jul 2026.
  - Competition is crowded: Wispr Flow note-taker (Aug 2026), Google AI Edge Foresight (8 Oct 2026), Superhuman acquired Fathom (Sep 2026), and others.

---

## 1. Platforms

| Platform | Status | Notes | Source |
|---|---|---|---|
| macOS desktop | Main platform. Requires macOS 14 per the setup guide; the troubleshooting doc says macOS 13+, best on 14.2+ | Electron app (trusted origin `app://ui/`; SQLCipher local DB). Auto-updates roughly every 10 min. Installers: `https://api.granola.ai/v1/download-latest`. Version endpoint: `https://api.granola.ai/v1/get-versions` (e.g. production 7.205.x). | [DOC setup](https://docs.granola.ai/help-center/getting-started/setting-up-granola-for-the-first-time), [DOC managed installs](https://docs.granola.ai/help-center/getting-started/managed-installations), [3P Strix, Jul 2026](https://www.strix.ai/blog/granola) |
| Windows desktop | Launched 11 Jun 2025 | Per-user installer. Mic permission is granted automatically, with no system-audio permission prompt. Some features are missing on Windows: screen snapshots, Slack-huddle chat message. "Meeting apps we don't support yet" on Windows may lose the other side's audio. | [CL 11 Jun 2025](https://www.granola.ai/updates), [DOC transcription issues](https://docs.granola.ai/help-center/troubleshooting/transcription-issues) |
| iOS (iPhone) | Launched 30 Apr 2025. Version 1.15.7 (14 Sep 2026). Requires iOS 18. App Store rating 5.0 from 14K ratings. | In-person capture, outbound phone calls, Lock Screen Live Activity and Dynamic Island, Lock Screen and Home Screen widgets, Siri support, Chat. Runs on iPad (no Chat on iPad) and Vision Pro (visionOS 2). | [App Store](https://apps.apple.com/us/app/granola-ai-meeting-notes/id6739429409) |
| Android | Launched 1 Jul 2026. Last updated 5 Oct 2026. Rated 4.8 from 3.08K reviews, 100K+ installs. | In-person capture. Ongoing "Taking notes" notification with pause, resume and end controls, plus a Quick Settings tile. No phone calls. Docs say no Chat, but the 5 Oct 2026 "What's new" lists "Use voice dictation in chats", which suggests Chat is arriving. Also new: attach or take photos in notes. No in-app subscriptions. | [Play Store](https://play.google.com/store/apps/details?id=ai.granola), [DOC mobile](https://docs.granola.ai/help-center/ios/getting-started) |
| Apple Watch | Launched 28 Jul 2026. Requires watchOS 11. Free. | Records on the Watch and syncs through the iPhone (details in §2.9). | [CL blog 28 Jul 2026](https://www.granola.ai/blog/granola-for-apple-watch), [DOC](https://docs.granola.ai/help-center/ios/apple-watch) |
| Web (notes.granola.ai) | View and edit only | **Cannot capture or transcribe.** Shared notes on the web show the summary only, with no transcript. There is a mobile-web download button. | [DOC transcription](https://docs.granola.ai/help-center/taking-notes/transcription) |
| Linux | None | | |

- **Sign-in:** Google, Microsoft (since 15 Jan 2026), or SSO. There are no passwords and no built-in 2FA; auth is delegated to the identity provider.
- **Multiple accounts:** desktop has an account switcher. Mobile allows one account at a time.
- **Calendar:** Google Calendar and Microsoft 365 / Exchange Online (no on-prem Exchange). **No Apple Calendar.**

---

## 2. Capture

### 2.1 Bot-free capture (desktop)
- **What it does:** captures the **microphone** and **system audio** (macOS: "Screen & System Audio Recording" permission) and streams them in real time to transcription providers. Deepgram and AssemblyAI are named on the security page.
- It works with **any** app whose audio plays through the computer: Zoom, Meet, Teams, Webex, Slack huddles, FaceTime, WhatsApp, VoIP, and even podcasts or YouTube.
- It **cannot isolate a single app**. It captures the combined system stream, so music played during a meeting gets transcribed too.
- **No audio or video is stored.** Audio is passed straight to the provider and deleted after transcription. There is no playback and **no importing of audio files** (e.g. MP3).
- Two channels are kept separate: mic = "Me", system audio = "Them". This is how Me/Them attribution works without diarization.
- **Mic selection:** pick from a list at the bottom of the transcript panel. **"Auto"** follows the meeting app's mic if one is open, otherwise the system default. Tip: for Bluetooth headsets, use the built-in mic so the headset stays in high-quality output mode.
- **VDI (Citrix/AVD):** not supported. In those environments the Teams desktop app doesn't pass other people's audio.
- Source: [DOC transcription](https://docs.granola.ai/help-center/taking-notes/transcription), [DOC transcription issues](https://docs.granola.ai/help-center/troubleshooting/transcription-issues), [security page](https://www.granola.ai/security).

### 2.2 When transcription starts (never fully automatic)
Transcription starts only when the user:
1. clicks a **notification** (calendar or ad-hoc);
2. opens a meeting from Home **after** its scheduled start time;
3. clicks **"➕ New Note"** (top right) for ad-hoc calls; or
4. is already sitting inside an upcoming meeting's note when its scheduled start time passes. It then **auto-starts at the start time**.

"Automatic start for all meetings" is an open feature request. Granola says it wants better transcript deletion and editing tools first. Source: [DOC transcription](https://docs.granola.ai/help-center/taking-notes/transcription), [DOC feature requests](https://docs.granola.ai/help-center/feature-requests).

### 2.3 Meeting detection and notifications
- These are **Granola's own pop-up windows** in the top-right, not native OS notifications. They are configured in-app under Settings > Notifications, where they can be turned off entirely or **per app**.
- **Calendar notification:**
  - Appears **1 minute before** each event that has **2 or more attendees**.
  - Clicking it **opens the video-call URL** from the event *and* opens the Granola note and **starts transcribing**.
  - It has a solid coloured bar on the left.
  - With Microsoft sign-in, you can join Teams calls straight from the reminder.
- **Ad-hoc call detection:**
  - Triggers when Granola detects the mic in use by another app.
  - Title varies by app: **"Huddle detected"** (Slack), **"Call detected"** (FaceTime/WhatsApp), **"Meeting detected"** (others).
  - The body names the app using the mic and has a **"Take Notes"** button.
  - It has a **dashed** line on the left.
  - If a call starts **within 15 min after** a scheduled calendar meeting, Granola assumes it is that meeting and shows the event name.
- **Window repositioning:** when a meeting starts, Granola repositions its window beside the call. Toggle: Settings > Preferences > "Reposition Granola for meetings".
- **Menu bar** (macOS):
  - Shows time left in the meeting, e.g. "10m left" (v6.399, 12 Dec 2025).
  - Optional **menu-bar calendar** showing the next meeting; click it for the day's schedule. Enable in Settings > Calendar ([3P createwith, ~Apr 2026](https://www.createwith.com/tool/granola/updates/granola-adds-menu-bar-calendar-view-for-quick-access-to-upcoming-meetings)).
- Source: [DOC notifications](https://docs.granola.ai/help-center/taking-notes/notifications), [CL 12 Dec 2025](https://www.granola.ai/updates/whats-new-2025-12-12).

### 2.4 Recording indicators
- **In a note:** **green "dancing bars"** at the bottom (three vertical bars) mean audio is being captured. Clicking them confirms the state.
- **Floating "live meeting indicator" (the "nub"):**
  - Appears on the **right edge of the screen** when another app is in front.
  - Click it to return to the note; drag the handle at its bottom to reposition it.
  - It also shows a snapshot icon or panel when screen snapshots are running.
- **Orange warning** when Granola can't reach the transcription servers ("having trouble transcribing").
- **Mobile indicators:**
  - iOS: Live Activity on the Lock Screen ("taking notes" plus duration) and Dynamic Island with a timer.
  - Android: ongoing notification with pause, resume and end.
  - Apple Watch: green screen, chime and haptic tap at start and end.
- Source: [DOC transcription](https://docs.granola.ai/help-center/taking-notes/transcription), [DOC mobile transcription](https://docs.granola.ai/help-center/ios/transcription).

### 2.5 Stop, pause, resume and auto-stop
- **Stop button** at the bottom of the note.
- **Pause and resume:**
  - In the transcript panel, **Stop/Resume** halts transcription temporarily without ending the meeting. Granola recommends pausing before discussing passwords or other sensitive details.
  - Mobile has pause and resume on the note-in-progress card.
- **Auto-stop** happens when:
  - the call has ended, judged from transcript length, whether the call app is still using the mic, and (for calendar meetings) the scheduled end time;
  - **15 minutes** pass with no new audio; or
  - the computer **sleeps**.
  - On macOS below 14.2, non-admin users don't get the "call ended" signal.
- **Ad-hoc notes** have no end time, so they rely on the mic-release signal. Granola may keep transcribing if audio continues.
- **Back-to-back meetings:** if Zoom keeps the mic open between calls, the two meetings can **merge into one transcript**. The workaround is to stop manually between them.
- **Resume after the meeting ended:** open the transcript panel and click **Resume**. New audio is appended and the notes can be regenerated.
- **Navigating away during a meeting:** press the **back button** to browse other notes while transcription continues (since 6 Mar 2025). **Esc** returns to Home even while transcribing (Dec 2025).
- Source: [DOC transcription](https://docs.granola.ai/help-center/taking-notes/transcription), [CL updates](https://www.granola.ai/updates).

### 2.6 In-person meetings
- **Desktop:** just open a New Note. The mic captures the room.
- **Mobile is designed for in-person meetings.** The phone mic (or a connected Bluetooth mic) captures the room.
- **Mobile transcription** uses temporarily cached audio and generates notes **after** the meeting, so it takes longer than desktop. The user gets a push notification when notes are ready.
- **Mobile speaker labels** are diarized as "Speaker A/B/…".
- **Mobile limits:**
  - Android: recordings under about 5 minutes may not generate notes.
  - All mobile: recordings of 10+ hours may fail.
- Source: [DOC mobile transcription](https://docs.granola.ai/help-center/ios/transcription), [security page](https://www.granola.ai/security).

### 2.7 Phone calls (iOS only, since 15 Sep 2025)
- **Setup:**
  1. Tap the phone icon (bottom-left).
  2. Enter your number.
  3. Granola **calls you** with a 6-digit verification code.
  4. Optionally import contacts (all or a selection).
- **Use:** dial a contact or number from **Granola's built-in dialer**. Granola acts as the "man in the middle" (an HN commenter's description), and the callee sees **your own number** as caller ID.
- **Inbound calls are not supported** because of iOS limits.
- After the call, notes generate in a few minutes and a notification is sent.
- **Country list:** roughly 40 countries, including the US, Canada, UK, EU and much of APAC. India-to-India calls and Telstra numbers are not supported.
- **One phone number per Granola account.** Settings > Phone Calls > Delete Phone Number frees it up.
- **Review complaints (Oct 2025):** UI lag toggling the speaker, notes generated even for unanswered calls, and call logs not grouped.
- Source: [DOC phone calls](https://docs.granola.ai/help-center/ios/phone-calls), [CL 15 Sep 2025](https://www.granola.ai/updates/introducing-notes-on-phone-calls).

### 2.8 Mobile (iOS and Android) capture UX
- **Ways to start a note:**
  - New note in the app.
  - Tap "start notes" on an in-progress meeting in the Coming up list.
  - Open an upcoming meeting and tap "start".
  - iOS: "Start Granola" from a Live Activity prompt.
  - iOS: Lock Screen or Home Screen widget.
  - Android: Quick Settings tile.
- **Note-in-progress card** at the bottom of the screen. On iOS, swipe up to open the editor and type rough notes. Markdown `###` headers and `*`/`-` bullets work.
- **Mobile can't:**
  - capture virtual meetings (the OS doesn't allow capturing other apps' audio);
  - create workspaces (it can switch between them);
  - create or apply templates;
  - share to integrations;
  - create or manage folders (it can add notes to existing ones);
  - change calendar settings;
  - view the trash (iOS).
- **Coming up on mobile** shows a short list of events; desktop shows the next 14 days.
- **Sync:** a note syncs to desktop **only after transcription and summarization finish**. Edits to completed notes sync both ways.
- Source: [DOC mobile](https://docs.granola.ai/help-center/ios/getting-started), [DOC sync](https://docs.granola.ai/help-center/ios/syncing-with-desktop).

### 2.9 Apple Watch (28 Jul 2026)
- **Ways to start:** open the app, use the watch-face **complication**, or tap the wrist when the **pre-meeting tap reminder** fires. Then pick an upcoming meeting or a new note.
- Screen turns **green**, a **chime** plays and the wrist gets a tap on start and end. The chime **can't be disabled**.
- **Recording behaviour:**
  - Keeps recording in the background, with wrist down and screen dimmed.
  - **Works offline**; a connection is only needed for syncing and generation.
- **Sync states:** Saved → Syncing to phone → Synced → Uploading → Generating notes → Complete.
- "**Watch notes**" list. Items under "**Waiting to sync**" can be retried with Sync or deleted.
- An interrupted recording (call, Siri, battery) can sync its **partial audio**, but recording can't be resumed.
- Source: [DOC](https://docs.granola.ai/help-center/ios/apple-watch), [CL blog](https://www.granola.ai/blog/granola-for-apple-watch).

### 2.10 Shared-screen snapshots (Mac only, macOS 14+)
- **What it does:** when someone else shares their screen, Granola takes **periodic snapshots of the shared window**, attaches them to the note, and **uses them to improve the enhanced notes**.
- **Enable:** Settings > Preferences > Features > "Take snapshots of shared screens". Needs Accessibility, then Screen Recording permission.
- **Enterprise:** off until an admin enables it under Workspace > Data security.
- **First-run offer:**
  - Beside the live indicator: "**Auto-capture shared screen**" with an **Enable** button.
  - Shown once a day for up to 3 days.
  - Closing it hides it for that meeting only.
- **Turning it off:** "**Turn off for this meeting**" on the note or indicator stops it for that meeting only.
- **Pausing:** snapshots pause when the meeting window isn't visible ("Paused screen capture").
- **Where snapshots go:** an "**Images**" stack at the bottom of the note, above chat. It says "Capturing" while running. Click to expand, click a snapshot for full size, hover and press **x** to remove.
- **Supported apps:** the doc lists Teams desktop and Teams in Chrome/Edge. It does not capture **your own** Teams share, Safari or Firefox. An earlier version said this was a beta/Labs feature with Teams "coming soon".
- **Consent reminders:** a consent card the first few times, then the short reminder "Always get consent when taking snapshots of shared screens".
- **Upload:** snapshots are uploaded to Granola.
- Source: [DOC](https://docs.granola.ai/help-center/taking-notes/capture-shared-screens).

### 2.11 Images and files in notes
- Drag-drop or paste **JPEG/PNG** into raw notes as extra context (since 4 Nov 2024). Android can now take or attach photos (5 Oct 2026).
- **File uploads** as context (since 5 Jun 2025):
  - In Chat via the attachment button. Not available in iOS Chat.
  - In folders. Limits: **30 MB per file**, **200 files per folder**, **1 GB per folder**.
- Source: [DOC file limits](https://docs.granola.ai/help-center/troubleshooting/increased-file-limits).

---

## 3. Notes UX (core "notepad")

### 3.1 Layout and editor
- **Home:**
  - "**Coming up**" (calendar events, 5 shown by default, page through with arrows).
  - "**Your notes**" list.
  - A "Suggested follow-up emails" section when relevant.
  - Sidebar with spaces and folders; toggle it with **⌘S / Ctrl+S**. It auto-closes when a note is opened and docks inline on wide windows.
- **Bottom-left sidebar icons:** People (person icon), Companies (building icon), and the workspace switcher (shows "See more" when you have more than 3 workspaces). Trash appears at the bottom of the sidebar once something is deleted.
- **Note editor:**
  - A blank Notion-like canvas.
  - Supports Markdown shortcuts: `#` headings, `-`/`*` bullets, `[]` checkboxes (in-note checklists, Jan 2026), `**bold**`, `*italic*`.
  - Formatting keys: ⌘/Ctrl+B, I, U. **⌘⇧V** pastes without formatting (Jan 2026).
  - Backspacing at the start of a heading reverts it to text, and Tab indents lists.
- **"Focus mode":** window chrome fades while typing ([3P](https://createwith.com/tool/granola/updates/granola-adds-focus-mode-to-reduce-distractions-while-taking-meeting-notes)).
- **@mentions** (Dec 2025): mention a teammate with `@`, with an option to **email them a link** to the note. Hovering a mention shows an **attendee card** with recent notes and quick actions.
- **Default note title** is "New note".
- **Dark mode** (Nov 2024). The font is SF Pro since Jan 2026 (was Inter).
- **Offline:** notes are readable and editable offline (Dec 2024), and search works offline. Notes are cached locally in an encrypted SQLCipher DB.
- **Meeting header:**
  - Attendees button with an attendee card (notes and sharing info).
  - Join-call button.
  - Event popover: change which calendar event a note is attached to. It suggests events from the past 24h.
- Source: [DOC Granola 101](https://docs.granola.ai/help-center/getting-started/granola-101), [DOC spaces](https://docs.granola.ai/help-center/sharing/folders/spaces-and-folders), [CL Dec 2025–Jan 2026](https://www.granola.ai/updates).

### 3.2 "Rough notes → AI-enhanced notes" flow
- **During the meeting** the user types anything (typos and abbreviations are fine) or nothing at all. Headings in the raw notes guide the structure of the output. **Granola treats anything the user typed as important to include.**
- **Trigger:** generation is **automatic** when the call-end is detected or the user stops. A notification fires when it's ready. There is also a manual **"Enhance notes"** button.
- **Inputs:**
  - the transcript;
  - the raw notes;
  - calendar event info (title, attendees);
  - the user's **profile** (role, company, focus areas — Settings > Profile);
  - the selected template;
  - screen snapshots, if enabled.
- **Models:** a blend, mainly OpenAI and Anthropic.
- **Visual distinction:** in the enhanced view, **the user's own words are black and AI-added content is grey**. The docs say raw notes are "highlighted in black"; the blog says added detail is in grey.
- **Two layers / tabs:**
  - **"Enhanced"** (AI notes).
  - The user's raw "**My notes**" (private notes), reached with the **three-horizontal-lines** icon at the top right.
  - Editing raw notes and regenerating takes the edits into account.
- **Citation and provenance:** a **magnifying glass** beside each enhanced bullet shows the transcript excerpt or raw note it came from. Shared-note viewers can **hover a point** to see transcript context.
- **Redaction:** Granola "tries its best to hide sensitive information" in notes automatically.
- **Edits are per-note.** They don't train future output; use templates or the profile for consistent style.
- Source: [DOC AI-enhanced notes](https://docs.granola.ai/help-center/taking-notes/ai-enhanced-notes), [DOC writing notes](https://docs.granola.ai/help-center/taking-notes/taking-notes-in-granola), [CL blog get-the-best (Aug 2026)](https://www.granola.ai/blog/get-the-best-from-granola).

### 3.3 Regenerating and editing
- **Template picker:** the ✨ button at the top-right of the notes, or the dropdown arrow on the "**Enhanced**" tab.
  - Choose a different template, which regenerates the notes.
  - Or press 🔁 (circular arrows) next to the current template to regenerate with the same one.
- **Confirmation dialog** before regenerating notes you've edited (Jan 2026).
- **Regenerate prompts** appear when attendees change, transcript chunks are deleted, raw notes are edited, or more transcript is captured.
- **"Edit by asking"** (24 Jul 2025, desktop):
  - Type into Chat ("make this more detailed", "Niall is actually spelt Nyal", "change £4.5m to £3.5m in the last section", "translate…").
  - Rewrite mode triggers on words like "rewrite". You can **select text first** to scope the edit.
- **Direct editing:** enhanced notes are fully editable rich text.
- **Retry button** on a failed generation uses a fallback LLM path.
- iOS notes can only get a new template or regeneration on desktop.
- Source: [DOC AI-enhanced notes](https://docs.granola.ai/help-center/taking-notes/ai-enhanced-notes), [CL edit by asking](https://www.granola.ai/updates/edit-notes-just-by-asking), [DOC notes not generating](https://docs.granola.ai/help-center/troubleshooting/notes-not-generating).

### 3.4 Templates
- **Default "Auto" template:** the default summarization adapts to the meeting type. You **cannot set a different default template** (per the feature-request doc).
- **Built-in templates:**
  - 29 shipped since Oct 2024 (e.g. project kick-off, pipeline review).
  - Names seen across sources: 1-on-1, Stand-up, Discovery/Customer Discovery call, Sprint Planning, Weekly Team, Interview Debrief, Project Kick-Off, Pipeline Review, Design Crit, Investor Update, Sales call, User interview, **Retrospective** (added 7 Aug 2026).
  - Which built-ins are **displayed depends on the user type picked at onboarding**; change it in the profile.
- **Applying a template:**
  - Type `/` in the note (the "/template" command, **17 Aug 2026**) to apply one **before or during** the meeting. Its formatting carries into the enhanced notes.
  - Or switch templates after generation via the Enhanced-tab dropdown.
  - Templates are **desktop only**.
- **Custom templates:**
  - Open from "All templates…" in the template menu, or "Manage templates" in the workspace menu.
  - A template is a prompt document: purpose and context, length and style instructions, plus **sections** with per-section instructions. A third party mentions a "Meeting Context" prompt field.
  - Editing a built-in is the easiest way to start a custom one.
- **Sharing templates:**
  - Private by default. Click "private to me" to share with your **organization**, defined as the **same email domain** (no sharing for Gmail/Yahoo users).
  - Workspace-shared templates also exist.
- **Template smart suggestions:** the Oct 2024 post said Granola suggests a template from title, description, attendee roles and past patterns. The current docs instead describe "Auto" plus manual choice.
- **Folder templates** (May 2025, 2.0): User Research, Sales Pipeline, Interview Loop. These pre-load folder structure, permissions and starter chat prompts.
- **Doc inconsistency:** the feature-requests page still says you can't choose a template before or during a meeting, but the templates doc and the 17 Aug 2026 launch say you can.
- Source: [DOC templates](https://docs.granola.ai/help-center/taking-notes/customise-notes-with-templates), [3P createwith /template 17 Aug 2026](https://www.createwith.com/tool/granola/updates/granola-adds-template-command-for-consistent-notes-during-meetings), [CL templates Oct 2024](https://www.granola.ai/updates/upgraded-meeting-note-templates), [CL 2.0](https://www.granola.ai/blog/two-dot-zero).

### 3.5 Action items
- There is **no task object**. Action items are plain text in the notes (often a "Next steps" section with owners). They can't be checked off, assigned or synced natively ([3P teardown, Jan 2026](https://meetingnotes.com/blog/granola-ai-teardown)).
- The only native checkbox is the Markdown `[]` in notes.
- To push action items elsewhere: Zapier (Linear, Jira, Asana…), MCP, the API, or Chat actions.

### 3.6 Deleting and restoring notes
- **Delete:** hover a note → ⋯ → "Move to trash". On mobile, swipe to delete.
- **Trash:**
  - In the sidebar, desktop and Android only.
  - Restore with the back arrow, or delete permanently.
  - **Auto-purged after 30 days.** Permanent deletion is irreversible.
- Source: [DOC](https://docs.granola.ai/help-center/taking-notes/deleting-notes).

### 3.7 Crunched (year in review)
- Launched **4 Dec 2025**: a "Wrapped"-style personal recap of the year built from your meeting notes.
- Only you see it, with optional sharing of individual insights. Retired in Jan 2026.
- Source: [CL](https://www.granola.ai/updates/granola-crunched).

---

## 4. Transcript

- **Transcript panel:**
  - Opened with the **waveform-bars icon** to the left of the "Ask anything" chat bar at the bottom of the note.
  - Shows a **chat-bubble layout**: **grey bubbles on the left** = system audio ("Them"), **green bubbles on the right** = microphone ("Me").
  - **Controls:**
    - show/hide
    - Stop/Resume
    - **Copy all** (copy icon at the top right)
    - **click any chunk** to copy or **delete** that chunk
    - **search** within the transcript
    - microphone selector
    - language switch mid-meeting
- **Speaker labels:**
  - **Default:** "Me" / "Them", from the mic vs system-audio split. The AI infers names from context in the notes.
  - **Speaker tags (real names):**
    - Desktop only (macOS and Windows), for **Google Meet** (Chrome Accessibility or the Granola Chrome extension), **Zoom** (Zoom Workplace desktop app, not the web client) and **Microsoft Teams** (desktop app, since 22 Sep 2026).
    - Works by reading the meeting app's **participant display names and active-speaker indicator** through OS **Accessibility APIs** (macOS needs the Accessibility permission; Windows doesn't).
    - Settings > Preferences > Speaker tags shows a row per app with an "Enabled" or "Set up" state.
    - Not retroactive. Can't split several people on one room device. Overlapping speech may be misattributed.
    - Admins can pre-grant Accessibility via MDM or push the Chrome extension via Google Admin.
    - Reportedly **on by default** for new accounts ([3P tl;dv, Aug 2026](https://tldv.io/blog/granola-review/)).
  - **Mobile:** anonymous diarization "Speaker A/B/…".
  - **API representation:**
    - `speaker.source` is `microphone` or `speaker`.
    - `speaker.attribution` is `me` or `them` (added Jul 2026).
    - `diarization_label` holds "Speaker A" on iOS.
- **Timestamps:** the API returns `start_time` and `end_time` per transcript item. The in-app UI doesn't show timestamps prominently (none are documented).
- **Correcting speaker names:** edit the notes directly, or ask Chat ("Niall is actually Nyal").
- **Editing transcript text is not possible.** Only **deleting chunks** is (since 28 Jan 2026, reviewed with SOAISEC Labs). After a delete, Granola prompts you to **regenerate** so the content is purged from the notes, and deleted chunks are excluded from Chat.
- **Where viewers see the transcript:**
  - Viewers of a **shared note** can't read the full transcript (hover context only).
  - **Folder collaborators in the desktop app** get the full transcript.
  - The web view never shows it.
- **Personal jargon:** up to **30 terms** (Settings > Preferences > Language). **Workspace jargon:** up to **50 terms**, admin-managed. These boost recognition only: no replacement and no acronym expansion. Jargon reportedly doesn't work in Multi-language mode.
- **Transcript auto-deletion:** see §10.
- **Viewing a transcript on mobile:** open the note → ⋯ → View transcript; it can be shared from there.
- **Review complaints:** the transcript box is small, with no scroll-to-top. On the free tier, copy and export lose formatting into one big block ([App Store review, Sep 2026](https://apps.apple.com/us/app/granola-ai-meeting-notes/id6739429409)).
- Source: [DOC transcription](https://docs.granola.ai/help-center/taking-notes/transcription), [DOC speaker tags](https://docs.granola.ai/help-center/taking-notes/speaker-attribution), [DOC Meet](https://docs.granola.ai/help-center/taking-notes/speaker-attribution-google-meet), [DOC Zoom](https://docs.granola.ai/help-center/taking-notes/speaker-attribution-zoom), [DOC Teams](https://docs.granola.ai/help-center/taking-notes/speaker-attribution-teams), [CL delete transcript parts](https://www.granola.ai/updates/delete-parts-of-transcript), [DOC jargon](https://docs.granola.ai/help-center/customising-granola/customising-transcription), [API intro](https://docs.granola.ai/introduction).

---

## 5. Granola Chat ("Ask Granola")

### 5.1 History of versions
- **16 Jan 2025:** Ask Granola **during** a meeting ("what did I miss?", "what should I ask next?").
- **11 Mar 2025:** chat about a person or company, and use multiple meetings as context.
- **8 May 2025:** **chat across all meetings** (Home "Ask Granola" button).
- **14 May 2025 (Granola 2.0):** folder chat with citations, plus a reasoning-model dropdown with models from OpenAI, Anthropic and Google.
- **30 Sep 2025:** redesigned Chat with **Recipes**.
- **21 Apr 2026:** **Chat rebuilt as an agent**:
  - faster;
  - works across your notes and transcripts, Team Space notes and notes shared with you;
  - inline citations;
  - tells you **which notes it referenced** ("Coverage Notes"); you can redirect it to notes it skipped.
- **May 2026 (iOS 1.13.3):** "Chat can now search your notes, connect ideas across meetings, **research online**…" (web search).
- **11 Jun 2026:** **Claude Fable 5** added as a model.
- Source: [CL updates](https://www.granola.ai/updates), [CL chat smarter](https://www.granola.ai/blog/granola-chat-just-got-smarter), [CL agents design (6 May 2026)](https://www.granola.ai/blog/how-granola-thinks-about-designing-agents), [App Store version history](https://apps.apple.com/us/app/granola-ai-meeting-notes/id6739429409).

### 5.2 UX and scope
- **Where Chat lives:** a floating chat bar at the bottom of the screen with an "**Ask anything**" placeholder. **⌘J / Ctrl+J** opens Chat. It can also open as an overlay (you can scroll the page with the overlay open).
- **Scope comes from where you chat** (no manual transcript toggle):
  - **Homepage:** all notes.
  - **Individual meeting:** before, during or after it; can also rewrite notes.
  - **Folder or space:** including My notes and the Team space.
  - **People or Company record** (desktop).
  - **Multi-select:** tick checkboxes next to meetings, Shift-click a range, or "Select all".
- **Pre-meeting brief:** shows at the top of the chat panel in a meeting note (see §7.1).
- **Context used:**
  - notes and transcripts in scope;
  - shared-with-me notes;
  - Team Space notes;
  - uploaded files;
  - Gmail (for actions and briefs, if connected);
  - web research.
- The docs (older line) state Chat doesn't know your email, tasks or to-dos. In practice it now does use Gmail for briefs and actions.
- **Agent retrieval:** search, shortlist, read, repeat. It prefers reading structured notes for about 100 meetings over full transcripts for about 10 (transcripts cost roughly 10x the tokens). Coverage notes show which sources were examined, and the user can ask it to do more.
- **Answers:** inline **citations** that jump to the source, with citation markers stripped when copying. Thinking models show a "**Thought for Xs**" expandable reasoning view.
- **Dictation:** a mic icon in the chat bar dictates a question. **X** cancels; **✓** sends. This is not meeting transcription. Android added "voice dictation in chats" on 5 Oct 2026.
- **History:**
  - **Chat history is private** and never shared, even on shared notes.
  - Threads can be deleted (hover → trash icon).
  - One review noted history disappearing on re-entry ([3P tl;dv](https://tldv.io/blog/granola-review/)).
- **Plan limits:**
  - Basic: Chat sees the **last 30 days** and uses the **Auto** model only.
  - Business: full history, plus manual choice of **standard** or **thinking** models from OpenAI, Anthropic and Google.
  - Enterprise: same as Business. Fable 5 is **off by default** until an admin enables it; when used, it isn't zero-data-retention (Anthropic keeps logs for 30 days).
- **Platforms:** desktop and iPhone. Not on iPad. Android per the docs, but see above.
- Source: [DOC chat](https://docs.granola.ai/help-center/getting-more-from-your-notes/chatting-with-your-meetings), [DOC model selection](https://docs.granola.ai/help-center/getting-more-from-your-notes/understanding-model-selection-in-granola-chat), [DOC dictation](https://docs.granola.ai/help-center/getting-more-from-your-notes/granola-chat-dictation-vs-transcription).

### 5.3 Recipes (saved prompts and slash commands)
- **Open:** type **`/`** in the chat bar to list recipes (default, mine and workspace). There is also a **Recipes tab** in the sidebar.
- **Recipes tab sections:**
  - **Discover**: by Granola and experts such as Lenny Rachitsky, Matt Mochary and Ridd.
  - **My recipes.**
  - **Workspace recipes.**
  - Search covers both titles and prompt text.
- **Creating a recipe:**
  - Name: used as the `/shortcut`.
  - Prompt.
  - Type, which is either:
    - **Single meetings**: available in individual meeting chats, live or after; or
    - **Multiple meetings**: available on Home, folders, People, Companies and multi-select.
    - A recipe is exactly one type; make two recipes to cover both.
  - Advanced: pick a model (Business and Enterprise).
  - **Preview** runs the recipe on one meeting or folder so you can iterate.
- **Sharing:**
  - Publish to the workspace, or share by link with **Private / Anyone in workspace / Anyone with link**.
  - Recipients can try it on their own notes, save it, or **Remix** it into an editable copy. Remix can also change the type.
  - Public library: recipes.granola.ai (`/r/<uuid>`), grano.la/recipes.
- **Static prompts only:** no variables, placeholders or substitution. You can chain several recipes in one thread (e.g. a "company context" recipe first).
- **Named recipes:**
  - Sep 2025: **Coach me**, **Prep me**, **Write a brief**, **Improve our sales process**.
  - Apr 2026: **How I have changed**, **How I like to work**, **Prep my day**.
  - Marketing page: Mochary-method leadership coaching, Lenny's PRD template, "suggests three things to improve your week", customer-feedback themes.
  - A third party counted about 40 prebuilt recipes (Jan 2026).
- Source: [DOC recipes](https://docs.granola.ai/help-center/getting-more-from-your-notes/recipes), [DOC writing recipes](https://docs.granola.ai/help-center/getting-more-from-your-notes/writing-effective-recipes), [CL recipes 30 Sep 2025](https://www.granola.ai/blog/say-hello-to-recipes), [chat page](https://www.granola.ai/chat).

### 5.4 Actions in Chat ("Workflows")
- **Desktop only.** Draft and send a **Gmail email**, post a **Slack message** (channels, group DMs, DMs), or **find a time and create a Google Calendar event**. The calendar action works only for attendees in the same Google Workspace domain.
- **Connections** are personal, under Settings → Connectors. **Outlook and Microsoft are not supported** for these actions.
- **Always a review step:** Granola shows an **editable draft card** in Chat, and nothing is sent until **Send** or **Create event** is clicked.
- **Email draft:**
  - To, CC, BCC, subject and body.
  - Gmail signature toggle.
  - **Highlighted placeholders** that must be filled before sending.
  - "**Save as draft**" to Gmail, with an optional Gmail **label**.
- **Slack draft:**
  - Shows the destination and message.
  - `@`/`#` mention autocomplete after 2 characters.
  - Warns if a Slack Connect destination includes external people.
- **Calendar draft:**
  - Suggested times.
  - Edit the time directly or **move and resize the event in a calendar preview**.
  - **Conflicts marked.** If not everyone is free, it shows the best options and who conflicts.
  - Checks for an existing event to avoid duplicates. Google Calendar sends the invites.
- Source: [DOC workflows in chat](https://docs.granola.ai/help-center/getting-more-from-your-notes/workflows-in-chat).

---

## 6. Organization

### 6.1 Workspaces
- **A workspace is the billing and tenant unit.** It holds notes, spaces, folders, templates and recipes.
  - Personal workspace = one member. Team workspace = several members.
  - One account can belong to many workspaces. Switch in the bottom-left.
- **When creating a workspace:** name and logo; an optional "**allow teammates with the same domain to auto-join**" setting; "**Allow workspace to be discovered**" by matching domains.
- **Inviting:** by email (comma-separated paste supported), by an **invite link**, or from "people you meet with regularly" (colleagues from past Granola meetings).
- **Roles:** Admin and Member. Members can upgrade the plan but not manage the subscription.
- **Moving notes:**
  - One note: ⋯ → Move to workspace.
  - All your notes and your solo folders: **Transfer notes to another workspace**, with a preview first.
  - Admins can block this under **Data export permissions**.
- **Leaving:** Danger zone → type "Leave [name]". You must belong to another workspace and can't be the sole admin.
- **Deleting a workspace:** transfer notes, remove all other members, cancel the subscription, then type the name.
- **Removed members** lose access to all their notes in that workspace. Notes they added to folders stay visible.
- **Account-to-account import:** moves (not copies) all notes, including those older than 30 days, from an account with a different email. Uses a 10-minute email code. Desktop only.
- Source: [DOC workspaces](https://docs.granola.ai/help-center/workspaces), [DOC import](https://docs.granola.ai/help-center/transfer-notes-between-accounts).

### 6.2 Spaces (25 Mar 2026)
- The sidebar has two default spaces:
  - **My notes:** private; all notes you created plus private folders shared with you.
  - **[Workspace name]:** the Team space, visible to the whole workspace. Admins automatically own all team-space folders.
- **Custom spaces (admins only):** Settings → Spaces, or "Manage spaces".
  - A space has a name, icon, description and members.
  - Used for departments, regions or projects.
  - Deleting a space doesn't delete its notes.
- **API access per space:** "Allow Granola API access" is **on by default for new spaces**. It controls whether **workspace API keys** can read the space.
- **Filtering:** folders can be filtered by company or person.
- **Space chat:** chat with any space.
- Source: [DOC spaces & folders](https://docs.granola.ai/help-center/sharing/folders/spaces-and-folders), [CL Series C](https://www.granola.ai/blog/series-c).

### 6.3 Folders
- **Creating a folder:** the **+** next to a space. Set a name, icon and description; the description also helps Chat focus.
- **Subfolders:**
  - **One level deep only.**
  - Create via ⋯ → Create folder, or drag one folder onto another (owners only).
  - Subfolders inherit the parent's space and sharing.
- **Adding notes:** "Add to folder", drag and drop, or multi-select (checkbox, Shift-click, Select all). A note can be in **many folders**.
- **Auto-add for recurring meetings:** adding a note from a recurring series prompts "**Auto-add future meetings?**". Auto-add sends future instances to that folder; unchecking the folder on any note in the series removes the rule.
- **Owner vs member permissions:**
  - Owners: add or remove any note, change sharing, add or remove members, nest the folder.
  - Members: add their own notes, add or remove members, leave the folder. They can't remove others' notes, change sharing, delete or nest.
- **Link access:** **Invited people only** / **Everyone at [workspace]** (same domain) / **Anyone with the link** (non-users can view on the web).
- **Desktop folder members see full transcripts.** Web viewers see summaries only.
- **Folder "Integrations" button**: sets **auto-post** rules for Slack, HubSpot, Attio and Zapier, plus webhooks and (on spaces) Granola API access. Rules apply only to notes added **after** the rule is enabled.
- **Earlier, possibly superseded features:**
  - **"Browse" view** of public folders in your domain (2.0, Business and Enterprise).
  - **Smart folder suggestions** after meetings (announced as "soon" in May 2025; current status unknown).
- **Still requested:** a default folder, auto-filing, and an "unfiled" filter.
- Source: [DOC spaces & folders](https://docs.granola.ai/help-center/sharing/folders/spaces-and-folders), [CL team folders 14 Jul 2025](https://www.granola.ai/updates/say-hello-to-team-folders), [CL 2.0](https://www.granola.ai/blog/two-dot-zero), [DOC feature requests](https://docs.granola.ai/help-center/feature-requests).

### 6.4 People and Companies (CRM-like entity pages) (8 Sep 2025)
- **Where:** the **person icon** (People) and **building icon** (Companies) at the bottom-left of the sidebar.
- **How it's built:** **auto-built from the calendar attendees** of notes created from calendar events. Enriched with **photos, job titles and company info** from external data sources.
- **What a page shows:** all notes involving that person or company, **including notes shared with you by teammates**.
- **Search bar** (Dec 2025).
- **Chat scoped to a person or company**, and multi-meeting recipes can run there.
- **Limitations:**
  - No manual add or edit.
  - People are missing if they weren't calendar attendees, the note wasn't calendar-linked, or they **declined** the event.
  - Wrong names or logos are fixed by contacting support.
- **Attendee cards** in the meeting header and @mention hovers show recent notes with that person.
- Source: [DOC](https://docs.granola.ai/help-center/people-and-companies), [CL 8 Sep 2025](https://www.granola.ai/updates/three-new-features-customer-relationships).

### 6.5 Other organizing views
- **"Shared with me"** (17 Sep 2025): at the top of the sidebar, listing every note shared with you. Shared notes also flow into People and Companies and into Chat.
- **Related meetings:** a recurring-meeting note shows previous instances. They're linked by the **recurring event ID OR an exact title match**.
- **Search:**
  - Global search finds notes **by person**, **by company**, and works **offline** (Dec 2024).
  - Transcript search is per-note, inside the panel.
  - Chat serves as semantic search.
- **Calendar view:**
  - There's no full calendar grid.
  - "Coming up" on Home shows the next 14 days on desktop, 5 at a time.
  - The macOS menu-bar calendar is optional.
  - Settings > Calendar toggles which calendars are visible. Declined events and OOO, Focus and Working Location blocks are hidden. Duplicates appear when a calendar is shared twice.
- Source: [DOC calendars](https://docs.granola.ai/help-center/getting-started/syncing-your-calendars), [CL shared with me](https://www.granola.ai/updates/shared-with-me).

---

## 7. Pre- and post-meeting intelligence

### 7.1 Briefs (pre-meeting) (20 May 2026)
- **When:** generated **overnight** for upcoming **external** meetings only (attendees outside your domain). Internal-only meetings get none.
- **Where:** opening the meeting note shows the brief **at the top of the chat panel**. Click to expand. It **hides itself once read**.
- **Format:** usually **2–3 bullets**: who you're meeting, what you discussed last time, and open threads or agenda points. **Every fact is cited**, and a section at the bottom shows the agent's reasoning steps.
- **Sources:**
  - previous Granola notes with the same people or company;
  - shared workspace notes;
  - **the web**, including LinkedIn;
  - the calendar event and agenda;
  - optional **Gmail** (connect via a "Connect Gmail" banner under the brief, or Settings > Connectors > Gmail).
- **No filler:** if there isn't enough information (e.g. a brand-new contact with no online footprint), no brief is shown.
- **Feedback:** a thumbs-down button.
- **Availability:**
  - Business and Enterprise only (the pricing table lists "Pre-meeting briefs" as Business and up).
  - **Google Calendar only** for now.
  - macOS, Windows and iPhone.
- Source: [DOC briefs](https://docs.granola.ai/help-center/taking-notes/pre-meeting-briefs), [CL blog](https://www.granola.ai/blog/briefs-prepare-you-for-your-next-meeting-as-you-join).

### 7.2 Suggested follow-up emails (around 8 Jul 2026)
- **Requirements:** a paid plan or trial, Google sign-in, and Gmail connected with **read and send** scope. The Gmail account must equal the sign-in email (no aliases).
- **On first connect:** Granola drafts your **2 most recent external meetings** straight away.
- **When a draft is generated:**
  - the meeting had **at least 1 external guest and no more than 10 people**;
  - drafts appear 1–2 minutes after notes finish;
  - Granola **stays quiet** when Slack is the channel, a colleague owns the follow-up, or a blocker comes before next steps.
- **Where drafts appear:**
  - the **"Suggested follow-up emails"** section on Home (shown for **3 days**);
  - a "**View draft**" prompt at the top of the note.
- **Draft editor:**
  - subject, body and formatting;
  - CC/BCC;
  - attachments, **25 MB total**, not persisted;
  - Gmail signature toggle;
  - placeholders to fill.
- **Sending:** **Send**, then **Undo within about 10 s**.
- **Draft persistence:**
  - Draft edits are stored **on the device only**.
  - Drafts vanish once sent or dismissed, or if a reply already happened.
  - A dismissed draft can't be restored.
- **Off switch:** Settings > Preferences > Features > "Suggested follow-up emails".
- Microsoft/Outlook is not supported.
- Source: [DOC follow-up emails](https://docs.granola.ai/help-center/taking-notes/follow-up-emails).

### 7.3 Other post-meeting features
- Chat-drafted follow-ups (§5.4). "Schedule follow-up meeting with Gmail" ([3P createwith, Apr 2026](https://createwith.com/tool/granola)).
- Old 2.0-era Slack auto-post included a "Chat with this meeting" button (May 2025).

---

## 8. Sharing and collaboration

### 8.1 Note sharing
- **Notes are private by default.** They are **never auto-shared to attendees**.
  - The share button is hidden until a summary exists (Jan 2026).
- **"Shared" button** (top-right):
  - shows who has access;
  - **Copy link** creates a unique URL (`notes.granola.ai/d/<uuid>`);
  - invite by email (comma-separated paste supported). Granola users see the note in "Shared with me"; non-users get an email.
- **Roles:**
  - **Viewer** (default): read; hover a point for transcript context; **use Chat on the meeting** (with their own private chat history).
  - **Collaborator**: can edit and apply templates.
  - Folder access does **not** grant Collaborator on notes inside the folder.
- **Access requests:** opening a locked note shows "**Request access**" with an optional message. The owner sees a banner with **Grant access**, in the app or on the web.
- **Viewers can't see the full transcript.** To share the transcript, add the note to a shared folder.
- **Sharing doesn't create a seat** or workspace membership.
- **Default link visibility:**
  - Set per user in Settings > Preferences > Data & sharing: **Anyone with the link** / **Only my company** / **Private**. It applies to **new notes only**.
  - "Anyone with the link" was the default as of Apr 2026, per The Verge via [3P letsdatascience](https://letsdatascience.com/news/granola-exposes-notes-publicly-by-default-1f1d2de9). No confirmed change since.
- **Enterprise sharing policy:**
  - Admins pick **Allow public links** / **Only people at your company** (labelled "Only approved domains" once extra domains are added) / **Restricted access** (no link, invite only).
  - It applies **retroactively** to all notes and folders, and users can't exceed it.
- **More options under the ⋯ share menu:**
  - **Copy notes** as **Markdown** (summary only).
  - **Send notes via email**, which opens a Gmail or Outlook draft.
  - Slack, Notion, HubSpot, Affinity, Attio, Zapier.
- **Slack link unfurls:**
  - Rich previews show title, attendees, date and a summary snippet. "Anyone with link" notes unfurl automatically.
  - Company-only or private notes need "Show for my links" or the admin's "Show for everyone's links" (Settings > Connectors > Slack).
- **Comments: none documented.** Collaboration means co-editing (Collaborator role), @mentions with email, folders and shared Chat context. A third-party teardown loosely mentions "comments and agendas" in folders; this is unverified.
- Source: [DOC sharing notes](https://docs.granola.ai/help-center/sharing/sharing-notes), [DOC sharing controls](https://docs.granola.ai/help-center/consent-security-privacy/sharing-controls), [DOC Slack](https://docs.granola.ai/help-center/sharing/integrations/slack).

### 8.2 Team features
- **Team workspace, Team space and custom spaces.**
- **User groups** (Enterprise): SCIM-synced or manual (Settings → Members → Groups). Grant a group **Viewer** or **Collaborator** on a space or folder; access updates as membership changes.
- **Shared templates and recipes.**
- **Shared Zaps:** "Share with your workspace".
- **Workspace jargon.**
- **Enterprise usage analytics:** active users, notes taken and top note-takers, **without note content**.
- **Admins cannot read members' private notes.**
- Source: [DOC user groups](https://docs.granola.ai/help-center/sharing/user-groups), [CL Series C](https://www.granola.ai/blog/series-c).

### 8.3 Auto-share rules
- **Folder-level auto-post** to Slack, HubSpot, Attio and Zapier ("Note Added to Granola Folder" trigger), and **folder webhooks**.
- Only notes added after the rule is enabled are posted.
- There is **no global "post every note" rule.** Granola says integrations will always involve an action such as adding a note to a folder.

---

## 9. Integrations

| Integration | Type | Behaviour | Plan | Source |
|---|---|---|---|---|
| **Google Calendar** | Native | Primary calendar synced by default; other calendars toggled in Settings > Calendar; changes reflected within minutes. Recurring IDs link related meetings. | All | [DOC](https://docs.granola.ai/help-center/getting-started/syncing-your-calendars) |
| **Outlook / M365** | Native (since Jan 2026) | Same as Google; Exchange Online only. No Briefs, follow-ups or Chat actions on Microsoft. | All | [CL](https://www.granola.ai/updates/sign-in-with-microsoft) |
| **Apple Calendar** | Not supported | Frequent request | — | [DOC feature requests](https://docs.granola.ai/help-center/feature-requests) |
| **Gmail** | Native connector | Briefs context, suggested follow-up emails, Chat email actions, Save as draft and labels | Business+ for briefs and follow-ups | [DOC](https://docs.granola.ai/help-center/taking-notes/follow-up-emails) |
| **Slack** | Native | (1) Share a note to a **channel** from the note, posting a link plus a summary. Private channels need the app added via `/` → "Add apps to this channel". (2) Folder auto-post. (3) Chat posts to channels, group DMs and DMs. (4) Rich unfurls. Requires Google Workspace or M365, not personal Gmail. Emails must match. | **All plans, including Basic** (the only Basic integration) | [DOC](https://docs.granola.ai/help-center/sharing/integrations/slack) |
| **Notion** | Native | One-click per note into a **Granola-created database** ("Use a template provided by the developer"). You can reorder, sort and add columns, but not rename or remove them or choose another page. **No bulk or auto sync** (use Zapier). | Business+ | [DOC](https://docs.granola.ai/help-center/sharing/notion) |
| **HubSpot** | Native (OAuth) | Per note: pick a contact, company or deal. Settings choose whether a **Meeting** (default) or **Note** object is created. Folder auto-sync matches attendees by **email**. Last-sync status shown in Settings. Requires Workspace or M365. | Business+ | [DOC](https://docs.granola.ai/help-center/sharing/integrations/hub-spot) |
| **Attio** | Native (OAuth) | Per note: Granola suggests the people or companies in the meeting and adds a note to that record. Folder auto-sync matches Company, People and associated Deals by email. | Business+ | [DOC](https://docs.granola.ai/help-center/sharing/integrations/attio) |
| **Affinity** | Native (**user-pasted API key**, one per user) | Suggests matching People and Company records by attendee email; you can review, remove or search more. **Only adds notes, never edits.** | Business+ | [DOC](https://docs.granola.ai/help-center/sharing/integrations/affinity) |
| **Salesforce** | Via Zapier ("advanced integrations e.g. Salesforce" listed as Enterprise on pricing) | Creates Opportunities, Contacts and Tasks | Enterprise / Zapier | [pricing](https://www.granola.ai/pricing), [integrations](https://www.granola.ai/integrations) |
| **Zapier** | Native app | Triggers: **Note Added to Granola Folder** (new notes only) and **Note Shared to Zapier** (manual, from the note sidebar). Config: name in Granola, share with workspace, folder, workspace. Test pulls 3 recent meetings. **Payload:** title, creator (name and email), attendees (name and email), calendar event (title and datetime), **My Notes** (raw private notes), **Summary** (Markdown), **Transcript**, **Link**. Folder Zaps require Collaborator or Owner. | Business+, desktop | [DOC](https://docs.granola.ai/help-center/sharing/integrations/zapier) |
| Via Zapier (listed) | Zapier | Gainsight, Mailchimp, Pipedrive, Salesforce, Asana, ClickUp, GitHub, Jira, **Linear**, Monday, Trello, Google Docs and Drive, OneDrive, OneNote, Greenhouse, Airtable… | Business+ | [integrations page](https://www.granola.ai/integrations) |
| **MCP clients** | Official connector listings | Claude, ChatGPT, Cursor, Figma, Gemini, Grok, Lovable, Manus, Microsoft 365 Copilot, Replit, v0, Bolt.new, Duckbill, Dreamer, plus Aug 2026 additions (Sauna/Wordware, PromptQL, deelan, Blomma, Rezonant, Mira), Delphi (10 Aug 2026), Poke, SuperMe, HeyGen, Factory, ChatPRD | MCP plan rules | [integrations page](https://www.granola.ai/integrations), [3P createwith](https://createwith.com/tool/granola) |
| **Chrome extension** | Native | Google Meet speaker tags and the Meet automated chat message (Chrome, Edge, Arc, Brave) | All | [DOC](https://docs.granola.ai/help-center/taking-notes/speaker-attribution-google-meet) |
| **Google Calendar add-on** ("Heads Up", Labs, 22 Dec 2025) | Workspace Marketplace add-on | Adds a "heads-up" conferencing link to events. Attendees first see a page saying "Heads up I'm using Granola" and must click **Join meeting**. Zoom variant exists. | Labs | [CL](https://www.granola.ai/updates), [Marketplace](https://workspace.google.com/marketplace/app/granola_for_google_calendar/385148968918) |
| **Cal.com app** | 3rd party | Listed on cal.com/apps/granola | — | [cal.com](https://cal.com/apps/granola) |
| **Glean connector** | 3rd party | Glean has a native Granola connector | — | [docs.glean.com](https://docs.glean.com/connectors/native/granola/) |

### 9.1 MCP server (launched 4 Feb 2026; updated Mar 2026)
- **Endpoint:** `https://mcp.granola.ai/mcp`, **Streamable HTTP**.
- **Auth:** browser **OAuth 2.0 with Dynamic Client Registration** (no client ID or secret).
  - **Enterprise-Managed Authorization (EMA):** Issuer `https://mcp-auth.granola.ai`, ID-JAG audience, scope `mcp`. Tested with Claude plus Okta Cross App Access.
  - Supports confidential clients via `private_key_jwt` and **CIMD**.
  - **No API-key or service-account access.**
- **Setup:**
  - Claude: Settings > Connectors > Granola.
  - ChatGPT: Settings > Apps > Granola, then `@Granola`.
  - Claude Code: `claude mcp add granola --transport http https://mcp.granola.ai/mcp`, then `/mcp` → Authenticate.
- **Tools:**
  - `query_granola_meetings`: Granola Chat as a tool, i.e. natural-language Q&A.
  - `list_meeting_folders` (paid): id, title, description, note count.
  - `list_meetings`: id, title, date, attendees; filter by folder on paid plans.
  - `get_meetings`: id, title, date, attendees, **private notes**, summary.
  - `get_meeting_transcript` (paid): raw transcript.
  - `get_account_info`: email and active workspace.
- **Scopes:** **Personal notes** (owned, shared directly, private folders shared with you) and **Public notes** (workspace-visible and Team space).
  - Basic: personal notes, last 30 days, no transcript or folder tools.
  - Business: personal and public.
  - Enterprise: admin chooses the scopes (Settings → Workspace → General → Apps & connectors → MCP access). There is also a **"Meeting transcripts"** off-switch that applies to everyone, admins included.
- **Workspace behaviour:** MCP follows the **active workspace** in the app and does not merge workspaces.
- **Rate limit:** about **100 requests/min** across tools.
- **Audit events:** `mcp.tool_used` (with self-reported clientName/version), `mcp.token_enabled`/`regenerated`/`disabled`.
- Source: [DOC MCP](https://docs.granola.ai/help-center/sharing/integrations/mcp), [CL MCP](https://www.granola.ai/blog/granola-mcp).

### 9.2 Public REST API (v1.0 Feb 2026 → v1.5 Sep 2026)
- **Base URL:** `https://public-api.granola.ai/v1`. Auth: `Authorization: Bearer grn_…`.
- **Personal API keys** (Business and Enterprise):
  - Settings → Connectors → API keys → Create new key.
  - Choose scopes: Personal and/or Public.
  - Editable scopes; can be revoked (moves to a Revoked section).
  - Enterprise admins restrict member scopes under "API access for members".
- **Workspace API keys** (admins, Business and Enterprise):
  - Belong to the workspace and **don't expire**.
  - Read public notes plus spaces with API access enabled.
- **Endpoints:**
  - `GET /v1/notes`: params `created_before`, `created_after`, `updated_after`, `folder_id`, `cursor`, `page_size`; returns `{notes, hasMore, cursor}`.
  - `GET /v1/notes/{not_xxxxxxxxxxxxxx}?include=transcript`.
  - `GET /v1/notes/{id}/transcript`: cursor-paginated (Aug 2026).
  - `GET /v1/folders`: includes `parent_folder_id`.
  - Webhook endpoint CRUD.
  - `GET /v1/audit`.
  - Legal holds.
- **Note object:**
  - `id`, `object`, `title`, `owner{name,email}`, `created_at`, `updated_at`, `deleted_at`, `web_url`.
  - `calendar_event{event_title, invitees, organiser, …}`, `attendees[]`, `folder_membership[]` (including ancestors).
  - `summary_text`, `summary_markdown`.
  - `private_notes_text` and `private_notes_markdown`: owner's key only (Sep 2026).
  - `transcript[]{speaker{source, attribution, diarization_label}, text, start_time, end_time}`.
  - Returns **413 `TRANSCRIPT_TOO_LARGE`** when the transcript is too big to inline.
- **Only notes with a generated summary and transcript are returned.**
- **Rate limit:** burst **25**, sustained **5 req/s (300/min)**, applied per user or workspace. Over the limit returns 429.
- **No sandbox.**
- Source: [API intro](https://docs.granola.ai/introduction), [API changelog](https://docs.granola.ai/api-reference/changelog), [DOC API keys](https://docs.granola.ai/help-center/sharing/integrations/granola-api).

### 9.3 Webhooks (Business and Enterprise)
- **Create:**
  - In-app: Settings → Connectors → Webhooks → "Set up a webhook". Choose notes and events, then the URL. The signing secret is shown once, with a test event and a "create compatible API key" option.
  - Per folder: Folder → Integrations → Webhooks.
  - Via the API: `POST /v1/webhook-endpoints {url, scopes, events?, folder_ids?}`.
- **Scopes:** `personal`, `public`, `workspace`. A workspace key must pass exactly `["workspace"]`.
- **Events:**
  - `note.generated`: first summary generated.
  - `note.edited`: summary edited or regenerated; `data.changed_fields`, currently always `["summary"]`.
  - `note.access_granted`.
- **Payload:** `{event_id, event_type, note_id, occurred_at}`. **No content**; fetch the note via the API.
- **Signing:** **Standard Webhooks** spec, using headers `webhook-id`, `webhook-timestamp`, `webhook-signature` (`v1,<base64 HMAC-SHA256>` of `id.timestamp.body` with the `whsec_` secret).
- **Delivery:**
  - 15 s timeout.
  - Retry on 408, 429, 5xx and network errors with exponential backoff **for 4 days**. 3xx and other 4xx fail permanently.
  - The endpoint is auto-disabled after 4 days of failures and the creator or admins are emailed. Missed events aren't replayed.
- Source: [DOC webhooks](https://docs.granola.ai/webhooks).

### 9.4 Audit API and Legal Holds API (Enterprise)
- **Audit API:**
  - `GET /v1/audit` returns the newest events first, from **1 year** of history. `page_size` is at most 30.
  - Filter `action` by prefix: `transcription`, `auth`, `mcp`, `document`, `workspace`, `data_export`, `integration.*_connected`, …
  - Events ordered by `collected_at`.
  - Up to **5 audit keys** per workspace, created under Settings → Connectors → Audit API keys.
  - Notable actions:
    - `transcription.started` and `.ended`, with the calendar event ID;
    - `document.viewed` (source: meeting_mode/homescreen/folder/nub/deep_link/web/watch/share_link) and `transcript.viewed`;
    - sharing changes;
    - logins with device info;
    - API key use and MCP tool use;
    - data exports and note transfers.
- **Legal holds:**
  - Preserve notes, transcripts and attachments for a whole workspace or named custodians (up to 100 per request; by email or `usr_` id).
  - Enabled on request. Up to 5 keys.
- Source: [DOC audit](https://docs.granola.ai/audit), [DOC legal holds](https://docs.granola.ai/legal-holds).

---

## 10. Privacy, security, settings and admin

### 10.1 Data handling
- **Audio:** never stored. It is cached only during transcription, then deleted from Granola and third parties. Mobile uses temporarily cached audio and transcribes after the meeting.
- **Stored data:** transcripts and notes, kept **indefinitely** unless a retention policy applies.
  - Hosted in a US **AWS VPC** (no EU, UK, CA or AU residency), encrypted at rest and in transit, backed up daily.
  - Cached locally on the device (offline use).
- **Subprocessors** include Deepgram, AssemblyAI, OpenAI, Anthropic and Google, under enterprise agreements with **no third-party training**.
- **Model training:**
  - **By default Granola trains on anonymized data** on Basic and Business. Opt out with Settings > Preferences > Data & sharing > "Use my data to improve models for everyone" (per account; not available on Android).
  - Enterprise is **opted out by default**, enforced workspace-wide.
- **Compliance:**
  - **SOC 2 Type II** (Jul 2025), GDPR and UK GDPR with a standard DPA (SCCs), **EU AI Act** alignment (enterprise page).
  - **HIPAA with BAA for Enterprise only**, covering desktop only.
  - Not ISO 27001, FERPA or NHS DSP Toolkit.
- **Bring-your-own-model:** not supported.
- **Vulnerabilities:** a disclosure policy with monetary rewards (no formal bug bounty). Public post-mortems:
  - AssemblyAI key exposure;
  - Google auto-join for unmanaged accounts;
  - a Google Workspace session-logout issue;
  - Tenable TRA-2025-07 (API info disclosure, May 2025);
  - **desktop navigation one-click account takeover** (Strix, reported 23 Jun 2026, fixed 29 Jun 2026; Electron `will-navigate` was unguarded and the preload bridge was exposed).
- Source: [DOC security FAQ](https://docs.granola.ai/help-center/consent-security-privacy/security-privacy-data-faqs), [DOC model training](https://docs.granola.ai/help-center/consent-security-privacy/model-training), [security page](https://www.granola.ai/security), [enterprise page](https://www.granola.ai/enterprise).

### 10.2 Transcript auto-deletion
- **Individual setting:** Settings > Preferences > Data & sharing > "Auto deletion period for transcripts".
  - Options: 1 day / 1 week / 1 month / 3 / 6 months / 1 year / Off.
  - A **1-week cooldown** applies before the first deletion, then deletion rolls on continuously.
- **Enterprise workspace policy:** set up by contacting Granola. It overrides individual settings, deletes older transcripts **immediately** with no cooldown, and is visible under Workspace > Data security.
- **Effects:**
  - Notes remain.
  - **Regeneration is impossible.**
  - Chat answers are limited to what's in the notes.
  - The transcript panel shows a "deleted" message.
- Source: [DOC](https://docs.granola.ai/help-center/consent-security-privacy/transcript-auto-deletion).

### 10.3 Transparency and consent features
- **Automated chat message:**
  - Granola posts a notice in the meeting chat once transcription starts **and** speech has been captured. In Zoom and Teams, the meeting window must be **in focus** and the chat composer empty.
  - **Google Meet** uses the Chrome extension, and the message is **pinned** where allowed.
  - **Zoom and Teams** work on macOS (Accessibility permission, which lets Granola "control your computer") and Windows.
  - **Slack huddles:** macOS only, posted in the huddle thread.
  - Only **one message** is sent when several workspace members use Granola in the same meeting. Its link opens a **live page listing who is transcribing**. A reminder is posted when someone joins late.
- **Granola Watermark:**
  - Installs a **virtual camera** (macOS camera extension, macOS 14+; Windows 11 virtual camera with an admin prompt) named "<Your Camera> - Granola Watermark".
  - Select it in Meet, Zoom, Teams, Slack or any app. It overlays a "Transcribing" badge **only while transcribing**.
  - A reminder appears in the meeting view until you switch cameras.
  - A troubleshooting check lives in Settings > Get help > Troubleshooting.
- **Admin enforcement (Enterprise):**
  - Admins can **require** these features. A "Setup required" card appears on Home.
  - Granola may **stop transcribing with a countdown** if the camera is off or the chat message fails.
  - Turning on the watermark requires at least one chat-message option first, as a camera-off fallback.
- **Defaults:** both features are **off by default** for individuals. They are set per user on Basic and Business, and admin-enforceable on Enterprise (pricing lists the org-wide notice as a "pilot").
- **Heads Up** (Labs, Dec 2025): see the Calendar add-on row in §9.
- **Platform notices** (Zoom disclaimer, Teams custom recording notice): customers configure these themselves.
- Source: [DOC transparency intro](https://docs.granola.ai/help-center/consent-security-privacy/transparency-solutions/introduction), [DOC admin](https://docs.granola.ai/help-center/consent-security-privacy/transparency-solutions/for-enterprise-admins), [DOC user Zoom/Teams](https://docs.granola.ai/help-center/consent-security-privacy/transparency-solutions/for-individuals/zoom), [DOC watermark](https://docs.granola.ai/help-center/consent-security-privacy/transparency-solutions/for-individuals/watermark).

### 10.4 Admin controls (Enterprise unless noted)
- **SSO (SAML/OIDC via a setup portal):**
  - Requires **50+ seats** and a company domain.
  - "**Enforce SSO login**" covers the whole domain.
- **SCIM directory sync and synced groups.** Synced members can't be removed manually.
- **Domain auto-join and discoverability** (all plans). Domain management consolidates free accounts.
- **Workspace sharing policy** (retroactive).
- **Data export permissions:**
  - "Allow exporting data" (Enterprise default **off**).
  - Move notes to other workspaces.
  - Transfer notes to other accounts.
- **Integration, API and MCP controls:**
  - MCP access scopes and a transcript off-switch.
  - API access scopes for members.
  - Per-space API access.
  - Admin controls for sharing, API, MCP and integrations.
- **Model controls:** org-wide model-training opt-out; Fable 5 toggle.
- **Feature toggles:**
  - Screen-snapshot toggle (Data security).
  - Transparency enforcement.
  - Workspace jargon.
- **Transcript retention policy; legal holds; Audit API.**
- **Usage analytics dashboard** (no note content).
- **iOS MDM / Intune app wrapping; HIPAA workspaces.**
- **MDM deployment:**
  - Pre-grant macOS Accessibility via MDM; push the Chrome extension via Google Admin; "Configure Granola with MDM" app settings.
  - Unauthenticated installer endpoints, plus `get-versions` for pinning.
- **Billing:** monthly or **annual (Enterprise only)**, via Stripe.
- Source: [DOC SSO](https://docs.granola.ai/help-center/sso-setup-guide), [DOC workspaces](https://docs.granola.ai/help-center/workspaces), [pricing](https://www.granola.ai/pricing).

### 10.5 User preferences (Settings map)
- **Profile:** name, job, context for the AI, **Generate CSV** export, import notes from another account, Delete account.
- **Preferences:**
  - Features: speaker tags, screen snapshots, suggested follow-ups.
  - Data & sharing: default link sharing, model training, transcript auto-delete.
  - Language: transcription language (English or Multi-language) and summary language (English or Auto); personal jargon.
  - Transparency: chat message and watermark.
  - Reposition window.
  - **Labs:** beta channel and experimental features such as Heads Up.
- **Calendar:** visible calendars, menu-bar calendar.
- **Notifications:** on/off and per app.
- **Connectors:** Slack, Notion, HubSpot, Attio, Affinity, Zapier, Gmail, Google Calendar, API keys, Workspace API keys, Webhooks, Audit API keys, Legal hold keys.
- **Workspace:** General, Members (and Groups), Spaces, Data security.
- **Billing:** upgrade, share a checkout link (valid 7 days), coupon, manage via the Stripe portal.
- **Get help:** Troubleshooting, with network connectivity and watermark checks.
- **CSV export details:**
  - Emailed within a few hours; the link is valid 24 h; one export per 24 h.
  - Includes title, summary, transcript and details for **owned notes with summaries**.
  - Includes notes older than 30 days even on Basic.
  - **No admin-wide export.**
- Source: [DOC export](https://docs.granola.ai/help-center/sharing/exporting-notes), [DOC profile](https://docs.granola.ai/help-center/customising-granola/profile-and-preferences).

---

## 11. Pricing (granola.ai/pricing, fetched 2026-10-08)

| | Basic $0 | Business $14/user/mo | Enterprise from $35/user/mo |
|---|---|---|---|
| Meeting notes | Unlimited ("Unlimited meeting notes for free") | Unlimited | Unlimited |
| History visible in app | **Last 30 days** (older notes stored, hidden until upgrade; CSV export still includes them) | Unlimited | Unlimited |
| Transcriptions per day | No limit | No limit | No limit |
| AI chat | Within and across meetings; Auto model only; 30-day scope | Auto plus manual standard and thinking models; Fable 5 | Same; Fable 5 needs admin |
| Templates, recipes, shared folders, speaker tags, bot-free capture, all apps (desktop, iOS, Android, Watch) | Yes | Yes | Yes |
| Pre-meeting briefs, follow-up emails, workspace dictionary | — | Yes | Yes |
| Integrations | **Slack only** | Notion, Zapier, HubSpot, Attio, Affinity | + "advanced" (e.g. Salesforce) |
| MCP | Personal notes, 30 days, limited tools | Personal and public notes, all tools | Admin-scoped |
| API and webhooks | — | Yes | Yes + Enterprise/Audit/Legal hold APIs |
| SSO/SCIM, admin controls, org-wide training opt-out, retention policy, sharing policy, usage analytics, HIPAA, MDM/Intune | — | — | Yes |
| Billing | — | Monthly only | Monthly or annual |

- **Programs:**
  - **Startups and students:** 12 months of Business free.
  - **Nonprofits:** 50% off.
  - Referral and affiliate programs.
  - A Lenny Product Pass code (new accounts only).
- **Trials:** new users get a time-limited Business trial (third parties say 14–30 days).
- 1.5% of revenue goes to Stripe Climate.
- **Outdated third-party claims to ignore:** "25 lifetime meetings" free cap, and an $18/mo "Individual" plan. Both reflect pre-2026 pricing.
- Source: [pricing](https://www.granola.ai/pricing), [DOC billing](https://docs.granola.ai/help-center/managing-your-account/subscriptions-and-billing).

---

## 12. Launch timeline (official changelog plus selected third-party)

| Date | Launch |
|---|---|
| 22 May 2024 | Public launch: AI notepad |
| 14 Jul 2024 | Auto-post notes to Slack |
| 7 Oct 2024 | 29 built-in templates, custom templates |
| 23 Oct 2024 | $20M Series A |
| 4 Nov 2024 | Images in notes |
| 29 Nov 2024 | Dark mode |
| 16 Dec 2024 | Fast search (by person or company, offline) |
| 18 Dec 2024 | Offline notes |
| 20 Dec 2024 | Team invites, Team & Billing |
| 16 Jan 2025 | Ask Granola during meetings |
| 12 Feb 2025 | Share to Notion |
| 6 Mar 2025 | Back button during meetings |
| 11 Mar 2025 | Chat about a person or company |
| 30 Apr 2025 | iOS app |
| 8 May 2025 | Chat across all meetings |
| 14 May 2025 | **Granola 2.0**: shared folders, folder chat, model picker, Browse; $43M raise |
| 22 May 2025 | 10 languages |
| 5 Jun 2025 | File uploads |
| 11 Jun 2025 | Windows app |
| 7 Jul 2025 | SOC 2 Type II |
| 14 Jul 2025 | Team Folders |
| 24 Jul 2025 | Edit notes by asking |
| 28 Jul 2025 | Zapier |
| 8 Sep 2025 | People and Companies, Attio |
| 15 Sep 2025 | iPhone phone calls |
| 17 Sep 2025 | Shared with me |
| 30 Sep 2025 | Recipes plus new Chat |
| 4 Dec 2025 | Crunched (year in review) |
| 12 Dec 2025 | v6.399: event suggestions, menu-bar time left |
| 19 Dec 2025 | v6.442: @mentions, trash, in-app calendar connect, People search |
| 22 Dec 2025 | Labs: Heads Up (Google Calendar add-on) |
| 8 Jan 2026 | Papercuts: regenerate warnings, attendee hover cards, SF Pro |
| 15 Jan 2026 | Sign in with Microsoft and Outlook |
| 16 Jan 2026 | v6.491: ⌘⇧V, Slack paste fixes |
| 28 Jan 2026 | Delete transcript chunks |
| 2 Feb 2026 | Rebrand (hand-drawn logo) |
| 4 Feb 2026 | **MCP** |
| Feb 2026 | Public API v1.0 |
| 25 Mar 2026 | **$125M Series C**: Spaces, nested folders, personal and enterprise APIs, MCP update, usage analytics, SCIM, consent management, scheduled transcript deletion |
| Apr 2026 | API v1.1 (folders) |
| 21 Apr 2026 | **Agentic Chat**, citations, new recipes |
| May 2026 | API v1.2 scopes; iOS Chat web research |
| 20 May 2026 | **Briefs** |
| 11 Jun 2026 | Claude Fable 5 in Chat |
| 1 Jul 2026 | **Android** |
| ~8 Jul 2026 | Follow-ups (suggested follow-up emails) |
| Jul 2026 | API v1.3 (me/them attribution) |
| 28 Jul 2026 | **Apple Watch** |
| 30 Jul 2026 | Class action filed (Chamberlain v. Granola) |
| 7 Aug 2026 | Retrospective template |
| 10 Aug 2026 | Delphi integration |
| 17 Aug 2026 | `/template` command (apply before or during a meeting) |
| Aug 2026 | API v1.4 (transcript endpoint) |
| 2 Sep 2026 | 32 languages (Multi-language mode) |
| 4 Sep 2026 | Automated CRM sync messaging (HubSpot, Attio, Affinity) |
| Sep 2026 | API v1.5 (private notes in API) |
| 14 Sep 2026 | iOS 1.15.7 |
| 22 Sep 2026 | Speaker tags for Microsoft Teams |
| 5 Oct 2026 | Android update: chat dictation, photos in notes, faster Home and folders |
| Undated (current docs) | Chat actions (Gmail, Slack, GCal), shared-screen snapshots, watermark, automated chat message, user groups, workspace API keys, webhooks, Audit and Legal Hold APIs, EMA for MCP |

Source: [granola.ai/updates](https://www.granola.ai/updates), [releasebot](https://releasebot.io/updates/granola), [createwith](https://createwith.com/tool/granola).

---

## 13. What users love (Reddit, App Store, reviews, HN)

- **No bot.** This is the most cited reason. "Seamlessly logged onto all my meetings without sending a bot" (Reddit, via [aitooldiscovery](https://www.aitooldiscovery.com/guides/granola-ai-reddit)).
- **Hybrid notes.** Typing a few rough bullets and getting a polished, personal note keeps you present. Notes are seen as better structured than "word salad" bot summaries (App Store 5★ review, Jun 2026).
- **Speed and minimal UI:** "minimal, fast, with no setup. It. just. works." Notes are ready seconds after the meeting.
- **Chat with transcripts** and cross-meeting queries ("what did we agree to?"). Users call it a killer tool for user research.
- **MCP:** power users like turning meeting history into context for Claude, ChatGPT and Cursor (App Store review).
- **Calendar reminder** makes starting effortless. Recipes such as "Coach me" are popular.
- **Privacy:** no audio stored.
- **Ratings:** App Store 5.0 (14K), Play 4.8, G2 4.7, Product Hunt 4.8.

## 14. Known complaints and missing features (as of 2026)

1. **Free plan history:** only 30 days are visible. This is the most common gripe. Older "25 lifetime meetings" complaints predate the current model.
2. **No audio playback or verification.** Errors can't be checked. Journalists and researchers call this a dealbreaker. Audio can't be imported.
3. **Speaker attribution:** historically "Me/Them" only, so multi-person calls collapsed into "Them". Now fixed for Meet, Zoom and Teams desktop via speaker tags. Still missing for Webex, Slack huddles, browser Zoom and Teams, and room devices.
4. **Silent capture failures:** sessions that looked live produced nothing, back-to-back meetings merged, and Android uploads failed and could only be deleted (Play review, Aug 2026).
5. **Accuracy:** names, numbers and jargon get mangled. Summaries can overstate decisions or turn tentative comments into action items.
6. **Integrations and export:**
   - No automatic Notion or Google Docs sync.
   - Notion sync is to a fixed database only.
   - Action items aren't real tasks.
   - No PDF, DOCX or JSON export (CSV only).
   - Copy-paste formatting loss.
   - Most integrations are paywalled; Basic has Slack only.
7. **Templates:** you can't set a default template (beyond Auto) or auto-apply per meeting type; you have to apply them by hand. `/template` (Aug 2026) partially addresses this.
8. **Organization:** no default folder, no auto-filing, no "unfiled" filter, subfolders one level deep only, and no manual People or Company records.
9. **Mobile parity:**
   - Android lacked Chat, phone calls and templates.
   - iOS can't record virtual calls or inbound calls.
   - The mobile app is described as "half-baked" (Play review, Sep 2026). An Android review also said it needs internet to record.
10. **Calendar:** no Apple Calendar. Briefs, follow-ups and Chat actions are Google-only.
11. **Privacy defaults:**
    - Training is on by default.
    - Link sharing was "anyone with the link" by default (Verge, Apr 2026).
    - Consent features are off by default.
    - US-only data residency, no ISO 27001, HIPAA only on Enterprise.
    - The **class action** (30 Jul 2026) leans on marketing such as "No bot. No notification."
12. **Chat:** answers can be thin, history can be lost on re-entry, and Chat is not on iPad.
13. **No editing of transcript text** (delete only). No auto-start for all meetings.
14. **"Attention debate"** on r/ProductivityApps: some users find they zone out because Granola is transcribing.
15. **Commoditization:** HN commenters say "the space feels commoditised". Competitors offer bot-free capture too: Circleback, Fathom, Fireflies desktop, Wispr Flow notes (Aug 2026), Google AI Edge Foresight (8 Oct 2026, on-device), and local/open-source tools such as Meetily, Hobnob, OpenWhispr, Muesli and Anarlog.

Sources: [tl;dv review (Aug–Sep 2026)](https://tldv.io/blog/granola-review/), [anarlog complaints (30 Aug 2026)](https://anarlog.so/blog/granola-ai-complaints/), [meetingnotes teardown (16 Jan 2026)](https://meetingnotes.com/blog/granola-ai-teardown), [aitooldiscovery Reddit roundup (2 Oct 2026)](https://www.aitooldiscovery.com/guides/granola-ai-reddit), [remio Reddit attention debate](https://www.remio.ai/post/reddit-users-are-turning-granola-into-the-new-meeting-attention-debate), [HN Apple Watch thread](https://news.ycombinator.com/item?id=49089026), [Play Store reviews](https://play.google.com/store/apps/details?id=ai.granola), [B&T on the class action](https://btlaw.com/en/insights/alerts/2026/what-the-granola-class-action-means-for-companies-building-and-deploying-conversation-capture-tools), [TechCrunch Google Foresight 8 Oct 2026](https://techcrunch.com/2026/10/08/google-releases-a-new-local-first-granola-competitor/). Several review sources are written by competitors (tl;dv, Anarlog, Kai, Circleback, Hedy), so weigh them accordingly. Direct Reddit access was blocked, so the Reddit material is secondhand.

## 15. Competitive positioning (brief)
- **vs Otter / Fireflies / Fathom (bot-based by default):**
  - Granola's edge: no bot, a hybrid human-plus-AI notepad, and "notes as the artifact".
  - Its weakness: no recordings or playback, and fewer sales-analytics features (call scoring, objection tracking).
  - Fathom has the most generous free tier. Fireflies is stronger on CRM and conversation intelligence. Otter leads on live shared transcripts and HIPAA.
- **vs Circleback:** both are bot-free. Circleback had earlier Android support and a different storage policy (recordings kept for 1 year).
- **vs Meetily and other local tools:** those run fully on-device with Whisper, are open source, and keep plain files. Granola is cloud-processed, more polished, and has team spaces.
- Sources: [Circleback comparison](https://circleback.ai/blog/best-ai-meeting-assistants), [Granola pricing comparison blog](https://www.granola.ai/blog/meeting-note-tool-pricing-granola-vs-fireflies-fathom-otter), [Meetily vs Granola](https://luci.memories.ai/blog/meetily-vs-granola).

---

## 16. Re-implementation notes (engineer's checklist derived from the above)

1. **Desktop capture service:**
   - Two separate audio streams (mic = "me", system loopback = "them"), streamed to a realtime STT over websocket.
   - Per-chunk records `{source, attribution, text, start, end, speaker_name?}`.
   - Mic picker with "Auto = follow the meeting app's mic".
2. **Meeting detection:**
   - Calendar poller, with a notification T-1 min for events with 2+ attendees. Clicking opens the conference URL and the note, and starts capture.
   - Mic-in-use detector naming the app, with per-app titles ("Huddle detected", etc.).
   - 15-min association window to the previous calendar event.
   - Auto-start at the scheduled time if the note is open.
3. **Auto-stop heuristics:**
   - Call app released the mic, plus transcript length, plus the scheduled end time.
   - 15-min silence. System sleep.
4. **UI:**
   - Floating draggable "nub" indicator, green bars, orange connectivity warning, menu-bar time left.
   - Transcript drawer: bubbles left grey / right green, search, copy all, per-chunk copy and delete, stop and resume.
5. **Editor:**
   - Markdown-ish rich text, images, checkboxes, @mentions with email notify, `/` menu for templates.
   - Raw notes vs Enhanced tabs.
   - Enhanced output preserves the user's own text (rendered black) and adds AI text (grey).
   - Each AI bullet links to a supporting transcript span (magnifier popover).
6. **Enhancement pipeline:**
   - Inputs: transcript, raw notes, calendar metadata, user profile, template (prompt plus sections), optional screenshots.
   - Regenerate with the same or a different template. Warn before overwriting edits.
   - Chat-driven partial rewrite of selected text.
7. **Speaker tags:** read the active-speaker and participant names from the Zoom/Teams/Meet UI via OS accessibility APIs (or a Chrome extension for Meet), and align by time with the "them" stream.
8. **Chat agent:**
   - Scope = current view (note / folder / space / person / company / selection / all).
   - Tools: search notes, read notes, read transcripts, web search, Gmail search, actions (draft email, Slack post, calendar event) with confirm-before-send cards.
   - Inline citations, coverage notes, model picker, `/` recipes (static prompts, single- vs multi-meeting type, share links, remix).
9. **Entities:**
   - People and Companies built from calendar attendees, enriched by an external lookup, aggregating own and shared notes.
   - Recurring-series linking by recurring event ID or exact title.
10. **Collaboration and organization:**
    - Workspaces → spaces (private, team, custom) → folders (1-level subfolders), with owner/member roles and link-access tiers.
    - Note-level Viewer and Collaborator roles. Access requests.
    - "Shared with me". Auto-add rules for recurring series.
    - Folder integration rules (Slack, CRM, Zapier, webhook).
11. **Briefs:** a nightly job per external meeting gathers past notes, web/LinkedIn and Gmail, and outputs 2–3 cited bullets, or nothing if there isn't enough.
12. **Follow-ups:** after external meetings (1+ external guest, 10 people or fewer), an LLM decides whether an email is warranted, drafts it, shows it in a Home section for 3 days, and sends via Gmail with a 10 s undo.
13. **Platform:**
    - MCP server (OAuth DCR, the 6 tools).
    - REST API (notes, transcript pagination, folders), Standard-Webhooks-signed webhooks, audit log, legal holds.
    - Transcript retention policies. Consent features (chat auto-message via accessibility, virtual-camera watermark).
