# Circleback (circleback.ai): complete feature inventory, as of 2026-10-08

Researched on 2026-10-08. The goal is to describe the product granularly enough to re-implement it. Transcription quality and language support are out of scope and appear only in passing.

## How to read this

**Source shorthand.** Every claim carries a source and a date.
- `DOC:` means `https://circleback.ai/docs/support/...` (the live help center, current as of 2026-10-08).
- `REL:` means `https://circleback.ai/releases/<slug>` (the official changelog, dated per entry).
- `API:` means `https://circleback.ai/docs/api/...` (the OpenAPI 3 spec is at `/docs/api/openapi.json`).
- Other URLs are written in full.

**Freshness.**
- The help center, the API docs, `/pricing` and `/releases` were fetched live on 2026-10-08. Treat them as authoritative for the current state.
- Third-party reviews dated before 2026-08-31 describe the old pricing (7-day trial, "Individual" at $20.83 and "Team" at $25). That pricing is now obsolete.
- Some older names have changed: "Workflows" became "Automations" on 2024-11-01. On 2025-10-31, what used to be called "Team" became "Workspace", and "Teams" are now sub-groups inside a workspace.

**Bulk sources.**
- Circleback publishes its docs in machine-readable form: `https://circleback.ai/docs/llms.txt`, `https://circleback.ai/docs/llms-full.txt` (all help articles plus the API reference) and `https://circleback.ai/llms.txt` (a product guide).
- The sitemap lists 219 release pages, from April 2023 to October 2026. Appendix A has a one-line digest of every one.

---

## 0. Company snapshot

- **Founding and funding:** Y Combinator W24. Founded in 2023 by Ali Haghani (ex-Stripe, where he led LLM work in support products; ex-Twitter) and Kevin Jacyna (ex-Tableau).
  - They wrote the first code in January 2023 and went full-time in July 2023.
  - The $2.5M seed round (2024-11-26) came from YC, Rebel Fund, Pioneer Fund and Transpose Platform. Source: https://circleback.ai/blog/seed-funding.
- **Size and metrics:** about 8 employees. Profitable since 2024 (self-reported). Self-reported run-rate revenue of more than $1M per employee. They don't spend on paid ads, so the free plan acts as the marketing budget. Source: https://runtimewire.com/article/circleback-free-tier-meeting-notes-marketing (2026-08-31).
- **Positioning:**
  - "Unbelievably good meeting notes" and "Be in the conversation, not in your notes."
  - Their differentiator is detailed, standalone notes plus auto-assigned action items, rather than a transcript-first product (TechCrunch, 2024-11-26).
  - Since 2026 they have positioned themselves as "company memory for people and AI agents", with the API, MCP and CLI built in.
- **Social proof:**
  - TIME named it a top pick for AI note-taking.
  - Claims "300+ YC companies" and "Top 5 most-used YC deal" (https://circleback.ai/startups).
  - Logos on the site: Runway, PostHog, Vercel, Box, Samsung, Deloitte, Accenture, Cisco, 3M, TikTok, Live Nation, Stanford, Harvard and others.
  - Garry Tan quote: "best AI note taker... action and todos tracking is the best in class."
- **Infrastructure hints:**
  - The meeting bot was built on **Recall.ai** as of the Launch HN on 2024-03-06 (https://news.ycombinator.com/item?id=39618640).
  - At that time, transcription used a "Whisper-based model".
  - The web app runs on Vercel serverless with Postgres behind the PgDog pooler (blog, 2025-10-09).
  - The desktop app is Electron, with a native Swift recording engine: ScreenCaptureKit on macOS and libobs ("OBSKit") on Windows (blog, 2026-04-16).
  - Primary processing happens in the US (DPA).

---

## 1. Plans and pricing (current, launched 2026-08-31)

Sources: https://circleback.ai/pricing, the embedded Stripe price JSON on that page, and REL:new-plans (2026-08-31).

| Plan | Monthly billing | Annual billing (per user/mo) | Included (cumulative) |
|---|---|---|---|
| **Free** | $0 | $0 | AI meeting notes and action items; Ask Circleback (assistant); view and draft emails; online and in-person meetings; bot **or** bot-free desktop app; mobile and Apple Watch apps; speaker-labeled transcripts; automatic tagging and sharing; **limited automations** (press reports say Slack and Linear are included); **30-day meeting and recording history**; API, MCP and CLI access; **2 teams**; unlimited meetings and members. |
| **Pro** | $18 | $15 ($180/yr) | + Unlimited meeting history; **1 year** of recording storage; automations with **all** integrations (Notion, HubSpot, Salesforce, Attio, webhooks and others); **custom AI insights**; personalized notetaker participant (custom bot name and image); 5 teams. |
| **Business** | $29 | $25 ($300/yr) | + Admin and member roles; unlimited **free viewers**; custom data retention; default and enforceable settings; **2 years** of recording storage; usage dashboard; unlimited teams. |
| **Enterprise** | (annual only) | $35 ($420/yr), with automatic volume tiers | + SSO (SAML/OIDC) and SCIM; invoice/PO billing; **unlimited** recording storage; priority support; HIPAA (BAA). |

**Enterprise volume tiers** (annual price per user): $420 up to 50 users, $360 up to 100, $300 up to 250, $240 up to 1,000, $210 up to 5,000, then $180. That works out to $35, $30, $25, $20, $17.50 and $15 per month. The page says: "Volume pricing is transparent... No sales call required."

**Other pricing terms:**
- **Discounts:** 30% for nonprofits (contact support), 50% for students (sign up with a school email), and a startup partner program with VCs and accelerators (https://circleback.ai/startups).
- **Free trial:** any paid plan can be tried free at signup. Free is "not a trial."
- **Billing rules** (pricing FAQ and DOC:billing/*):
  - Plan changes are prorated, and any remaining balance becomes a credit.
  - Cancellation takes "a single click": downgrade to Free.
  - Viewers are free seats.
  - If someone who already has their own paid subscription is added to a workspace, that subscription is automatically canceled and refunded, so billing consolidates.
  - Deactivated members free their seat at the end of the billing cycle.
- **Referrals:** a free month for both sides, through a referral link or through meeting-notes emails, which are tracked automatically. On teams, each referral earns a free seat for a month (DOC:billing/referrals). Affiliates earn 30% commission, "$75+ per customer" (https://circleback.ai/affiliates).
- **Payment methods** (DOC:billing/update-payment-method): cards; Apple Pay, Google Pay, Cash App Pay, Amazon Pay and Link; Klarna; ACH, Canadian PAD, Bancontact, iDEAL and bank transfer; Kakao Pay; stablecoins.
- **Invoice billing** (Enterprise, REL:switch-to-invoice-billing, 2026-09-14):
  - Path: Settings → Billing → ⋯ next to Payment method → Switch to invoice billing.
  - Takes effect at the end of the current billing period.
  - Invoices are emailed with net-30 terms and show as **Unpaid** under Invoices.
  - The workspace "may be limited" if an invoice goes unpaid.
  - The billing owner can "Add seats by invoice": choose a number of seats, the amount is prorated, and the seats are added immediately.
  - Billing information (email, company name, tax ID, address) is editable by the billing owner. Edits apply to future invoices only.
  - An "Email receipts" toggle is available.
- **API rate limits by plan** (API: overview): Free allows 3 requests per second and 20 per minute. Pro, Business and Enterprise allow 20 per second and 300 per minute.

**Pricing history, for context only (obsolete):**
- Until 2026-08-30 there was no free tier, just a 7-day trial that required a card.
- "Individual" cost $25/month, or $20.83/month billed annually. "Team" cost $25/user/month billed annually.
- The founders said the trial was causing significant drop-off, so they switched to a free plan where history is the paywall.

---

## 2. Platforms

| Platform | Status and details | Source |
|---|---|---|
| **Web app** | Full product. The original app lived at app.circleback.ai; current links use circleback.ai/meetings/{id}. A mobile-web layout and "Add to Home Screen" existed in 2024. | REL:improved-mobile-web-experience (2024-04-10) |
| **macOS desktop** | Electron shell with a native Swift capture engine. Requires macOS Sonoma 14.2+, 8GB RAM, Apple Silicon or Intel i5 8th gen+, and 50GB free disk recommended. Includes a menu bar app. | DOC:recording/desktop-app/desktop-recording-requirements; REL:desktop-app (2024-09-16) |
| **Windows desktop** | Requires Windows 11+, 8GB RAM, i5 8th gen or Ryzen 5 3000+, and a DX11 GPU. Includes a system tray app. | same |
| **iOS / iPadOS** | Launched 2025-02-19. App Store id 6738971048, "Circleback - AI Meeting Notes" by Circleback AI, Inc. Rated 4.8★ from 256 ratings; version 3.9.42 as of 2026-10-07; requires iOS 16.4+. Also runs on visionOS, and there is an **Apple Watch app** (2026-06-29). | REL:ios-app; https://apps.apple.com/us/app/circleback-ai-meeting-notes/id6738971048 |
| **Android** | Launched 2025-04-07 as `ai.circleback.app`, last updated 2026-10-08. | REL:android-app; Google Play |
| **Apple Watch** | Records in-person meetings straight from the wrist with no iPhone required. Has a watch-face complication for one-tap recording. | REL:apple-watch-app (2026-06-29) |
| **Slack app** | Assistant via @mention, DMs and the Slack agent panel; automation destination; Slack huddle recording. | §9 and §11 |
| **Raycast extension** | Record and search meetings from Raycast (2025-09-09). Raycast is also an MCP client. | REL:raycast-extension |
| **ChatGPT app** | The Circleback ChatGPT app is built on the MCP server (2026-03-09). | REL:circleback-in-chatgpt |
| **Claude, Cursor, Codex, Claude Code, Raycast** | Connect through MCP. | DOC:platform/circleback-mcp |
| **CLI** | `npm i -g @circleback/cli`, which installs the binaries `cb` and `circleback` (2026-03-18). | REL:circleback-cli |

**Localization:** the marketing site comes in many locales (fr, es, it, pt, nl, de, ja, ko, zh, fa, he, ar, fi, sv, th, vi, id, da and others). Product UI localization is not documented.

---

## 3. Capture modes

### 3.1 Calendar connection (prerequisite for most capture modes)

Source: DOC:account-and-setup/connect-your-calendar.

- **Where:** Settings → Calendar → Connect, for **Google Calendar** or **Microsoft Outlook Calendar** (OAuth).
- **Accounts:** you can connect multiple Google and Outlook accounts. Only the **primary calendar** of each account is read; shared and secondary calendars on that account are not visible. (Older note: "secondary calendars" support shipped 2024-07-10, meaning you could connect an additional calendar account.)
- **Microsoft admin consent:** an IT admin logs in, connects Outlook and ticks "Consent on behalf of your organization." The alternative is to set Entra "Allow user consent for apps" (DOC:enable-circleback-for-a-microsoft-organization).
- **Matching:** the calendar is used to match *any* recording (bot, desktop, mobile or import) to an event, which supplies the title, participant names and context.
- **Upcoming meetings on Home:**
  - A list of the day's and upcoming events, each with a join toggle.
  - The right-hand details panel shows invitees with name, profile picture, email, role, company website and RSVP status (REL:meeting-details-panel, 2024-07-17), plus the event description (2025-06-06).
  - Right-click or ⋯ offers "Email invitees", tag, and open in Google Calendar or Outlook (REL:open-meeting-calendar-event, 2025-01-15).
  - **"Review last meeting":** if you've met the same attendees before, an icon on the upcoming meeting opens the previous meeting (REL:review-last-meeting, 2024-02-09).
  - **Recent meetings with the same invitees** appear in the details panel (2024-08-02).

### 3.2 Meeting bot ("Circleback" participant), automatic via calendar

Sources: DOC:recording/bot/how-to-get-circleback-to-join-a-meeting, DOC:account-and-setup/meeting-joining-preferences.

- **Supported platforms:** Zoom, Google Meet, Microsoft Teams and Webex. Slack huddles work through a separate mechanism (§3.4). Zoom webinars that require registration work if the link has a `tkquery` param (REL:support-for-zoom-webinars, 2024-06-12). The homepage FAQ also lists GoTo.
- **Detection:** Circleback auto-detects calendar events that contain an online meeting link. It "join[s] or request[s] to join the meeting when it starts."
- **Global joining preferences** (Settings → Calendar → Joining preferences):
  - **Meeting-type toggles:**
    - **Internal:** everyone invited shares the calendar's email domain. On personal-email accounts this toggle is labeled **"All meetings"**.
    - **External:** at least one invitee has a different domain.
    - If both are off, the bot never auto-joins.
  - **Filters** (ANDed on top of the type toggles):
    - **Organized by someone else:** if off, the bot only joins meetings you organized. (Originally shipped as "only join meetings hosted by you", 2024-05-01.)
    - **Declined or pending:** if off, the bot only joins meetings you accepted. (Originally the "Accepted meetings" toggle, 2023-08-29.)
- **Per-meeting override:** toggle any upcoming meeting on or off from Home (or from the desktop menu bar, mobile or the API). This overrides the global rules in either direction.
  - For **recurring meetings**, the toggle opens a modal asking whether to apply it to *all occurrences* or *just this one* (REL:choose-joining-preferences-for-recurring-meetings, 2025-07-29).
- **Calendar event API fields** (API: GET /calendar/events): `willRecord` (bool), `willRecordReason` (string, for example "External attendees"), `overrideShouldRecord`, `isExternal`, `isHostedByMe`, `meetingPlatform` (for example `google_meet`), `platform`, `location`, `organizerEmail`, attendee `status` (accepted, declined, tentative or not_available) and `meetingId` (the Circleback meeting once one exists).
- **One bot per meeting:** if several users (from any workspace) have Circleback set to join the same meeting, only **one** bot joins. Its name and chat message list the people or workspaces it is joining on behalf of, and each user still gets the meeting in their own account (REL:one-circleback-participant-per-meeting, 2024-06-16).
  - Within a workspace, that produces **one unified meeting record**, and all members who enabled Circleback get owner access (REL:unified-meeting-records-for-teams, 2024-06-21).
- **Bot identity:** you can customize its **display name and image** under Settings → Preferences → Meeting participant. This is "Personalized notetaker participant" on Pro+ (DOC:customize-meeting-participant). Admins can set a workspace default or enforce it.
- **Chat message (consent and announcement):** when the bot joins, it posts a short intro in the meeting chat saying:
  - whether audio and video recordings are being stored, and
  - whether participants will receive the notes by email.
  - **This message cannot be disabled.** When recording storage is off, the message says that no audio or video is being stored.
  - There is no documented spoken audio announcement.
- **Waiting room:** the host may need to admit the bot. If the platform's waiting-room timeout expires before admission, the join fails.
- **Troubleshooting causes** (DOC:recording/bot/troubleshoot-circleback-not-joining-a-meeting):
  - Zoom "Only authenticated users can join."
  - Webinar "Registration Required."
  - Google Meet "Anyone with the meeting link can ask to join" turned off, or Workspace Meet safety settings blocking users who aren't signed in.
  - Internal-only org restrictions.
  - A stale meeting password in the invite.
- **Fallback to desktop:** if the desktop app has "Detect meetings" on and the bot can't join (for example, it isn't admitted), the desktop app falls back to recording locally, "so you never miss a meeting" (DOC:recording/desktop-app/record-meetings-with-the-desktop-app).

### 3.3 Meeting bot, manual invite by link

- **How:** in the left sidebar, open the dropdown next to **New meeting**, choose **Invite Circleback**, paste the live meeting URL and click **Add Circleback**. The bot "joins within a few seconds."
- **Calendar association:** a meeting added by link can be associated with a calendar event to pick up its title and invitees (REL:associate-linked-meetings-with-calendar-events, 2025-03-05).

### 3.4 Slack huddles

Sources: DOC:recording/bot/use-circleback-in-slack-huddles; REL:slack-huddles-support (2024-02-14, beta at launch).

- **Setup:** Settings → General (Workspace) → Slack huddles → Connect.
  1. Enter the Slack workspace URL.
  2. Circleback gives you a **dedicated email address**.
  3. Invite that address to Slack as a regular member. If the workspace uses SSO, invite it as a multi-channel guest.
  4. Add that member to any private channels you want covered.
- **Behavior:**
  - Circleback auto-joins huddles in any channel it belongs to, public or private.
  - In **DM huddles** it does not auto-join; someone has to use Invite people → Circleback.
  - Requirement: huddle participants must be in your Circleback workspace and use the same email in Slack and Circleback.
  - Output is the same as any meeting: notes, action items, transcript, and a recording if enabled.

### 3.5 Bot-free desktop recording (macOS and Windows)

Sources: DOC:recording/desktop-app/*; REL:desktop-app (2024-09-16) and later entries.

- **What it captures:** microphone, **computer (system) audio**, and optionally **video of a screen or window**. It works with any app: Zoom, Meet, Teams, Webex, Discord, RingCentral, phone apps and so on (https://circleback.ai/desktop).
- **Starting a recording manually:**
  - From the app: **New meeting** in the sidebar.
  - From the menu bar or system tray: **New Meeting**.
  - The recording screen auto-associates the **calendar event closest to now**. A chevron next to "New meeting" switches to a different event, and this can be changed even mid-recording (REL:change-calendar-event-while-recording, 2026-03-02, on desktop, mobile and web).
- **Recording settings** (adjustable before and **during** a recording, since 2026-03-09):
  - Microphone picker.
  - Screen or window picker, or None for audio only.
  - Computer audio toggle, recommended for online meetings.
- **Automatic microphone switching:** when the input device changes mid-meeting (headset plugged in, AirPods removed), recording continues on the new device without a gap (REL:automatic-microphone-switching, 2026-04-08).
- **Automatic meeting detection** (Settings → Notifications → **Detect meetings**):
  - When any app starts using the microphone, Circleback shows a notification offering to record.
  - **Ignored apps list:** apps whose mic use should never trigger the prompt (REL:ignore-specific-apps-when-detecting-meetings, 2026-06-17).
  - Recording notifications on desktop shipped 2025-09-16. You can set preferences in the notification and record with one click. If auto-start or auto-end is on, a notification appears before the recording starts or ends.
- **Start recording automatically:** after a detection, a **short countdown** runs and then recording starts. Clicking **Don't Record** cancels it.
  - History: on 2025-03-13 this was calendar-based (a 30-second countdown at the event's start time, only if the bot isn't joining). On 2025-07-21 it switched to "on microphone use".
- **End recording automatically:** recording stops when the mic is **no longer in use** (you left the call). **Keep Recording** overrides this.
  - History: before 2026-03-03, auto-end used the calendar end time or a period of extended silence, with a 30-second countdown.
- **Record meeting video:** on by default for detected meetings, capturing the **meeting window**. Toggle it in Settings → Notifications; the notification itself can also override it per meeting (REL:choose-whether-to-record-meeting-video-on-desktop, 2026-03-05).
- **Record panel:**
  - Originally a floating panel opened from the menu bar (2025-03-06).
  - Since 2026-05-29 it is an **always-visible minimal panel** shown for the whole recording. It expands on hover to show controls (pause, stop, mic and window selection), can be dragged anywhere and snaps into place.
  - Clicking the Circleback icon opens a **note-taking window** for private notes during the meeting.
  - Dark mode support (2025-06-05).
- **Menu bar / system tray** (DOC:use-circleback-from-the-menu-bar):
  - Shows today's meetings.
  - Clicking a meeting lets you open it in Circleback, **join the call** (for example "Open Google Meet"), view the calendar event, **email invitees**, or toggle bot joining.
  - **New Meeting** starts a recording.
  - Also: Open Circleback, Settings, Quit.
  - A "Show Meetings with No Invitees" toggle hides solo events from the list **and from the menu bar countdown** (REL:hide-solo-meetings-from-the-menu-bar, 2026-02-11). The countdown means the menu bar displays a countdown to the next meeting.
- **Notifications for upcoming meetings:** one-click "join and record". If the bot is already set to join, the desktop app does not prompt you to record.
- **Opening links:** Circleback meeting links open in the desktop app via a browser prompt, with an option to always do so (REL:open-meeting-links-in-desktop-app, 2024-12-31, v1.0.2).
- **Speaker labeling for desktop recordings:** participants start as "Participant 1, 2, ..." identified by voice. You rename them in the Transcript tab (REL:desktop-app, 2024-09-16). Voice profiles and calendar names then auto-label future meetings (§6).
- **Permissions:**
  - macOS: Microphone, and Screen & System Audio Recording.
  - Windows: Microphone, and Screenshots and screen recording. Mic **Exclusive Mode** must be off (DOC:troubleshoot-recording-on-desktop).
- **Reliability engineering** (blog, 2026-04-16, https://circleback.ai/blog/how-we-rebuilt-our-electron-recording-engine-in-swift):
  - Capture moved out of the Electron renderer into native Swift.
  - macOS: three ScreenCaptureKit sources are assembled into a file, with a lockstep audio mixer. If one source stalls for more than 500ms, the mixer switches to single-source mode. Sample-rate-lying drivers are corrected with a confidence-based crossfade.
  - Windows: libobs graph, with WGC capture falling back to BitBlt and **black-frame detection**.
  - **Fragmented MP4** segments are written locally and streamed to the cloud at the same time. A crash loses at most about 1 second, and uploads resume after network loss.
  - "Atomic" bridges Swift `@Published` properties to Jotai atoms.
  - Earlier: "streamed recordings", incremental upload to the cloud so a browser crash doesn't lose the recording (REL:streamed-recordings, 2024-08-01).
- **Web recording:** you can also record from the browser (the original "Record" feature, 2023-11-14). Recording starts instantly with no spinner (2025-02-21).

### 3.6 Mobile in-person recording (iOS and Android)

Sources: DOC:recording/mobile-app/record-meetings-with-the-mobile-app, plus releases.

- **Basics:** audio only, from the phone mic. Open **Record** in the bottom navigation, optionally pick the calendar event (the current one is pre-selected), and start.
- **Background:** recording continues when the phone is locked or you switch apps.
  - iOS shows a **Live Activity** on the Lock Screen and Dynamic Island, plus a paired Apple Watch, with **pause, resume and finish** controls (REL:recording-live-activity-on-ios, 2025-03-24; controls added 2025-06-05).
  - Android shows a live notification with the same controls.
- **Interruptions:** an incoming phone call may pause the recording. On iOS, if Background App Refresh is off, uploads can be slower.
- **Offline recording:** you can start and finish recordings offline. They auto-upload the next time the app opens with connectivity (REL:offline-recording-on-mobile, 2025-02-27).
- **Quick start:**
  - **iOS Lock Screen and Home Screen widget:** one tap starts recording the **current calendar meeting** (2025-12-11). The site's llms.txt says the widget exists on Android too.
  - **iPhone Action button:** Settings → Action Button → Controls → Circleback → Record. Pressing and holding the button opens Circleback and records the current meeting.
- **Speaker ID:** in-person recordings get "the same automatic participant voice identification."
- **Other mobile features:**
  - Add private notes before or during a recording (2025-09-30).
  - Edit names, notes and action items; changes auto-save (2025-05-28).
  - Share (link, workspace access, email notes) (2025-08-13).
  - Search (2025-05-30) and Assistant (Ask tab) (2025-08-13).
  - Synced transcript playback: the transcript auto-scrolls and highlights the current word, and tapping a word seeks to it (2025-10-22).
  - Background audio playback (2025-02-27).
  - Dark mode, following the system setting or set manually (2026-03-11).
  - View upcoming meetings and toggle bot auto-join.
  - View and manage action items.

### 3.7 Apple Watch

- **Recording:** a standalone watch recording works with **no phone nearby**. A complication on the watch face gives one-tap recording. The meeting flows into Circleback with notes, action items and transcript (REL:apple-watch-app, 2026-06-29; App Store listing).

### 3.8 Phone calls

Source: DOC:recording/mobile-app/record-a-phone-call.

- Circleback **does not record calls directly**. You use the OS call recorder and then import:
  - **iPhone** (iOS 18.1+, in select regions): during the call tap More → Call Recording, which announces itself to all parties. The recording lands in Notes → Call Recordings. From there, tap Share → Circleback.
  - **Android:** use Google Phone's **Record** (or Samsung's "Record call"), then in call history tap Share → Circleback.

### 3.9 File import (upload)

Sources: DOC:meetings/import-a-meeting; REL:increased-import-limit (2024-08-07); REL:import-call-recordings-on-mobile (2026-07-08).

- **Web or desktop:** New meeting dropdown → **Import** → choose an audio or video file, **up to 2 GB** with no length limit. Supported formats include MP3, MP4, WAV, M4A, MOV and other common types. Optionally associate the file with a calendar event to get the right title and names. The old limit was 500MB or 130 minutes.
- **Mobile:**
  - New-meeting button → (optionally tap the meeting name to pick an event) → ⋯ → **Import**.
  - Or use the **share sheet** from any app (Voice Memos, Files, Notes, call recorders), which imports automatically.
  - The meeting appears in the list while it uploads and processes.
- **Calendar association:** imports can be associated with calendar events, which enables voice identification and better automation matching (2025-02-06).

### 3.10 Import from other notetakers ("Switch to Circleback")

Sources: DOC:account-and-setup/switch-to-circleback/*; REL:import-meetings-from-other-apps (2026-08-28).

- **Where:** Settings → **Import**. Pick the source app, connect it, **preview the meeting count**, then click **Import**. The import runs in the background and you get an email when it finishes.
- **Supported sources:**
  - **Fathom:** OAuth; imports notes, transcripts, attendees, action items and recordings.
  - **Fireflies:** you paste a Fireflies API key; imports notes, transcripts, attendees and recordings.
  - **Granola:** OAuth.
  - **Read AI:** OAuth; only meetings you own. Recordings come across only if Read AI's "Downloads" setting is enabled.
- **Scope:** each person imports their own account's meetings. Imported meetings **keep the same sharing** as in the source app, and they are searchable and askable.
- **Other sources:** use `POST /api/meetings` (§12).

### 3.11 Recording storage toggle

Source: DOC:recording/configure-whether-meeting-recordings-are-saved.

- **Setting:** Settings → Preferences → "Save meeting audio and video recordings". It is on by default today.
  - History: recordings used to be off by default and only the transcript was kept (2023–2024).
  - Recordings were deleted after 30 days, then retention was raised to 365 days (2024-02-16), then to **2 years** (2026-01-30).
  - Current per-plan storage: Free gets 30 days, Pro 1 year, Business 2 years, Enterprise unlimited.
- **When off:** notes, action items, transcript and insights are still generated. The audio and video are discarded, so there is no playback or download.
- **Admin control:** workspace admins can set a default or enforce this setting.

---

## 4. Email connection (context source)

Source: DOC:account-and-setup/connect-your-email; REL:connect-your-email (2025-10-28).

- **Setup:** Settings → Connected accounts → **Gmail** or **Outlook Mail**.
- **What it's used for:**
  1. A "Recent conversations" history per person or company that mixes meetings and email threads.
  2. Note generation pulls **recent emails with the meeting's invitees** as extra context.
  3. Assistant and search cover emails as well as meetings.
- **Privacy:** emails are accessed **on demand** (at note generation or assistant query time) and **are not stored**.
- **Email drafting and sending** (REL:draft-and-send-emails, 2026-07-31):
  - Ask the assistant to write an email. It drafts using meetings, past threads, calendar events and **availability**, looks up recipients, and **replies in an existing thread** where appropriate.
  - The draft opens in an editor for review, then you hit Send to send it from Circleback.
  - When viewing a thread inside Circleback, you can also **Reply** manually.
  - The earlier version (2025-06-17) only pre-filled recipient, subject and body, then handed off to your email app.
- **CLI and MCP:** both can search connected email (§12).

---

## 5. Generated notes: structure and behavior

Sources: the public example meeting https://circleback.ai/view/m9vtzdx5fpzle2iypnm (Apr 2025); the webhook and API examples; REL:updated-meeting-notes-editor (2025-04-23).

- **Timing:** notes are generated automatically after the meeting ends. Reviewers report roughly 5 minutes (Dupple, 2026). No live, in-meeting AI notes are documented; private notes act as the live scratchpad.
- **Default structure.** The notes are Markdown, and headings use `####`:
  1. **Action Items** block at the **top** of the Notes tab. Each item shows an assignee avatar, title and description, and has a check-off box.
  2. `#### Overview`: 3–6 bullets summarizing the whole meeting.
  3. Several `#### <Topic title>` sections, one per discussion topic, in roughly chronological or topical order, each with bullets. Names are attributed ("Ali demonstrated...").
  4. **Private notes** sit below the AI notes, visible only to their author (§7.2).
  5. A **table of contents** in the top-right jumps between headings.
- **Notes style:** detailed prose bullets "designed to give someone who missed the meeting its context, decisions, and next steps" (compare page). There is no dedicated "Decisions" section; decisions appear inside topic bullets and the overview.
- **Inputs to note generation:**
  - Transcript with speaker names.
  - Calendar event title, attendees and description.
  - Your **private notes**, which steer emphasis (for example, spellings of names, or "this topic was the most important").
  - **Recent emails** with invitees, if email is connected.
  - **What was shown on screen** (slides, dashboards, docs): since 2026-05-28, details from screen shares go into the notes and are searchable and available to the assistant and MCP (REL:notes-capture-whats-shared-on-screen). This requires video capture (bot or desktop window recording).
  - **Custom vocabulary** (Settings → Preferences → Vocabulary; applies to future meetings; names and company names are included automatically).
- **Output language:** defaults to the transcript language. Settings → Preferences → language forces notes, action items and insights into a fixed language.
- **Customization levers.** There is **no user-editable notes template or per-meeting-type notes prompt** in the docs as of 2026-10:
  - The format is fixed.
  - Verbosity was adjusted per customer on the backend in 2024 (HN), and self-serve verbosity control was "planned" but has not shipped.
  - Per-meeting-type customization happens through **AI insights in automations** (§10.3), which produce extra structured outputs in an **Insights** tab rather than altering the main notes.
- **Editing:** the meeting name, notes and action items are inline-editable.
  - The editor (2025-04-23) has a TOC, a redesigned toolbar, keyboard shortcuts and links.
  - To edit before notes go out by email: turn off automatic invitee emails, edit, then use Share → Email to send manually.
- **Copying:** ⋯ next to the meeting name → copy **notes**, **transcript** or **insights** to the clipboard.
- **Export:** account-wide export under Settings → Account → Export produces notes, action items, insights, transcripts and recording download links. You get an emailed link when it's ready.

---

## 6. Speaker identification

| Mechanism | Behavior | Source |
|---|---|---|
| Bot meetings | Speaker names come from the meeting platform, so labels are reliable. | HN 2024 |
| Calendar-based names | Invitee names from the calendar event become candidate speaker names, and rename dropdowns are populated from attendees. | DOC:update-meeting-participant-names |
| **Voice profiles** | When you rename a participant in a meeting **associated with a calendar event**, their **voice is saved** and auto-recognized in future meetings **they are invited to**. | REL:automatic-participant-voice-identification (2024-11-20) |
| **Hybrid and same-device diarization** | Several people on one device (a conference room or shared laptop) are separated into distinct participants. You can rename them, and their voices are remembered. | REL:automatic-participant-identification-for-hybrid-meetings (2025-08-06) |
| Desktop, mobile and imports | Voice-diarized as "Participant N" until named or matched by voice profile. | REL:desktop-app |
| Rename UX | In the Transcript tab, click a participant name, then type or pick from the dropdown. You choose whether the change applies to **this segment only** or **all segments** for that speaker (2025-10-28). Changes propagate to the transcript, notes, action items and insights. | DOC + REL:edit-speaker-names-for-individual-transcript-segments |
| Visual cues | Each participant gets a unique color in the transcript (2023-12-15). The participants panel shows **speaking duration %** on hover (2024-06-11, 2024-07-17). | REL |

---

## 7. Meeting page anatomy and per-meeting features

### 7.1 Tabs and panels

- **Header:**
  - Meeting name, which is editable.
  - Tags (**+ Add tag**; hover over a tag to remove it).
  - **Share** button.
  - ⋯ menu: copy notes, transcript or insights; Automations (run or rerun); open the calendar event; Delete.
- **Notes tab:** action items, AI notes and private notes (§5).
- **Transcript tab** (REL:transcript-tab-updates, 2024-05-29):
  - Video or audio player on the right, with **Download** below the player. External link viewers cannot download.
  - Real-time word highlighting and auto-scroll, plus a "Scroll to now" button.
  - Click any word to seek.
  - **In-transcript search** (⌘F, 2024-10-07; upgraded 2026-05-25): every hit is highlighted in context, Enter or Shift+Enter or the arrows move between hits, a counter shows your position, and you can **search by speaker name**.
  - Highlight text to get a popup with **Comment**, **Share moment**, **Create clip** and **Copy**.
- **Insights tab:** results of AI-insight automations for this meeting.
- **Clips tab:** clips created from this meeting.
- **Right details panel**, resizable and collapsible with `]`:
  - **Assistant tab** (per-meeting chat).
  - Participants with speaking time.
  - Invitee details.
  - Recent meetings with the same people.

### 7.2 Private notes ("Add your own notes")

Source: DOC:meetings/add-your-own-notes; REL 2025-08-14.

- **When:** you can add them before, during or after a meeting. Open an upcoming or in-progress meeting to type.
- **Visibility:** they stay **private even when the meeting is shared**, and they appear under "Private notes" below the AI notes.
- **Effect on AI notes:** they feed the AI notes. Uses include a live scratchpad, questions to ask, and flags for importance.
- **Where they can be written:**
  - On desktop, from the record panel's note window.
  - On mobile.
  - Through the API, CLI and MCP as `privateNotes` on a calendar event before the meeting starts, with Markdown.
  - Through the assistant ("Add questions about our hiring plan to my private notes for tomorrow's board meeting").

### 7.3 Comments

Source: DOC:meetings/leave-comments-on-a-meeting; REL 2024-03-13.

- **How:** highlight transcript words, click **Comment**, and threaded replies follow.
- **Notifications:** **all workspace users with access** get an email for each new comment.
- **Visibility:** comments are only visible to workspace members, never to external link viewers. Commented spans are highlighted, and clicking one opens the thread.

### 7.4 Moments (deep links)

Source: DOC:meetings/share-meeting-moments.

- **How:** highlight transcript text and click **Share moment**. A link is copied that opens the meeting at that timestamp, with transcript and recording.
- **Access:** the recipient must already have access to the meeting.

### 7.5 Clips

Source: DOC:meetings/create-a-meeting-clip; REL:meeting-clips (2025-05-31).

- **How:** highlight a transcript range, click **Create clip**, name it, and click **Create**. The clip is a segment of the recording tied to that transcript span.
- **Location:** clips are listed in the meeting's **Clips** tab.
- **Sharing:** right-click or ⋯ → share, either with the workspace or externally via a public link.
- There is no documented global clips library or highlight reel.

### 7.6 Playback, download and copy

- Playback sits beside the transcript, with click-to-seek.
- Mobile has synced transcript playback.
- You can download the recording's audio or video file.

### 7.7 Delete and restore

Source: DOC:meetings/delete-a-meeting; REL:restore-deleted-meetings (2026-07-02).

- **How to delete:** ⋯ → Delete, or right-click on the Meetings page.
- **Recently deleted:** deleted meetings go to **Recently deleted** (your name in the bottom-left → Recently deleted) and stay there **30 days**. From there you can restore, or delete permanently right away.
- **Permanent deletion** removes notes, action items, transcript and recording.
- **Who can delete:** only owners (per the API).

### 7.8 Ownership

- **Multiple owners:** a meeting can have several owners, such as everyone in the workspace who had Circleback join.
- **Claim ownership:** if **all owners are deactivated**, any workspace member with access can open the share modal and use the "Make me owner" banner (2025-12-23).
- **Request access:** opening a link to a meeting you can't see shows a **Request access** button. Owners are notified and can share it with you (REL:request-access-to-meetings, 2026-08-10).

---

## 8. Action items

- **Auto-extraction** (since 2023-07-24):
  - After every meeting, Circleback creates action items for tasks that came up.
  - Each item is **auto-assigned** to an attendee when the conversation makes ownership clear; otherwise it is unassigned.
  - Each item has a **title** and a 1–2 sentence **description** with context. Reviewers frequently praise this.
- **Data model** (API, webhook):
  - `id` (int), `title`, `description`, `assignee` (profileId, name, email, title, companyName, or null), `status` ∈ {`PENDING`, `DONE`}, and the meeting link.
  - **There is no due-date field** in the API, webhook, CLI or MCP schemas. Marketing copy mentions "deadlines", and these likely end up in the text.
  - There is no priority field.
- **Manual actions:**
  - Add action items to a meeting (2024-07-12).
  - Edit title and description.
  - Reassign by clicking the assignee.
  - Check off, with a small celebration animation.
  - Create **standalone action items not linked to a meeting** (API, CLI, MCP and assistant; 2026-10).
- **Action Items page** (left nav, 2023-11-30):
  - A workspace-wide list covering you, your team and the people you met with.
  - **Filters:**
    - Assignee scope, mirrored by the API `assigneeType` enum: Me, NotMe, a specific Profile, MyWorkspace, OutsideMyWorkspace, Unassigned, Anyone. You can also filter by team.
    - Status.
    - **Tags** (2025-07-02).
    - **Companies** (2026-05-19).
    - Attendees.
    - Date range (CLI).
  - Filters **persist** across sessions (2024-06-19).
  - Press `F` to search filter options (2026-01-07).
  - Save filters as **Views** (tabs; 2026-03-19).
  - Click the linked meeting for context.
  - **Bulk selection** (2026-04-03): click, Shift-click a range, ⌘/Ctrl+A for all, or click-drag. The bottom action bar or ⌘K command menu lets you change status, reassign or delete.
  - ⋯ → **Mark all mine done** (2025-04-17).
- **Completion from outside the app:**
  - **From the notes email:** recipients tick a checkbox next to *their own* items, and **no Circleback account is needed** (2025-06-16).
  - **Limited viewers and viewers** can mark *their own* items done.
  - Through the assistant ("mark my action items from this morning's standup as done"), Slack, mobile, the API, CLI and MCP.
- **Syncing to task tools:**

  | Tool | How | Details |
  |---|---|---|
  | **Linear** (native automation) | Choose a Linear team. | Issues are created **assigned when the action-item assignee's email matches a Linear team member**. Two toggles, both on by default: "Issues assigned to a Linear team member" and "Issues not assigned to a Linear team member" (no assignee, or an assignee outside Linear). |
  | **monday.com** (native) | Choose a board and group. | One item per action item, assigned to the relevant person. The internal/external filter shipped 2025-04-14. |
  | **HubSpot** (native) | Action items become **HubSpot tasks**. | Assigned if the assignee is a HubSpot user (2025-04-30). |
  | **Attio** (native) | Creates **Attio tasks** linked to people or companies. | Assigned if the assignee is a workspace member (2024-09-05). |
  | **Notion** (native) | A database with one row per action item, or included in a page. | §10 |
  | **Asana, Jira, Todoist, ClickUp, Trello, Airtable** | Through **Zapier** or **Make**, or through the **assistant connector** (Asana: "Create tasks for the action items from this meeting in the launch project"). | — |
  | **Slack** | Post action items to a channel. | — |

- **Two-way sync:** there is no documented sync back from Linear, Asana or similar tools; completion status doesn't flow back. Reminders: only marketing mentions "reminders"; the docs describe none beyond the notes email and weekly roundup.

---

## 9. Search and "Ask Circleback" (Assistant)

### 9.1 Global search

Releases from 2024-08-02, 2024-09-09, 2024-11-14, 2025-05-28, 2025-06-30 and 2026-02-26.

- **Opening it:** a search bar at the top of the app, `/`, ⌘K or Ctrl K.
- **Query types:**
  - Meeting names.
  - "Something someone said" (moments and mentions).
  - Questions, which get AI answers.
  - **Invitee emails or domains**: typing `initech.net` returns every meeting with someone @initech.net. Several terms are space-separated.
  - Combinations, for example "demo initech.net".
- **Results page:** a "View all results" page with Meetings and Moments filters.
- **Scoping with @:** type `@` to add **tags**, **people**, **companies** or **specific meetings** as filters.
- **Quick navigation** (2026-02-26): the same modal navigates the app. Type "calendar", an automation name, an insight name or "new meeting" and press Enter to go there.
- **Past chats** can also be found from the search modal.

### 9.2 Ask page (cross-meeting assistant)

Sources: DOC:assistant/ask-circleback-assistant; REL:circleback-assistant (2025-07-17).

- **Search behavior:** it "agentically search[es] every meeting you have access to." That includes other people's meetings shared with you, and **emails** if connected. The Ask view isn't limited to your own meetings.
- **Citations:** answers come with **sources**. Hover a source for the specific references, and click it for full context, which jumps to the moment in the meeting. Per-meeting sources shipped 2025-03-25.
- **Filters:** `@` mentions for people, companies, tags and meetings.
- **History:** a history icon in the top right lists and searches past chats, and you can continue any chat.
- **Streaming responses** (2025-01-28).
- **Calendar questions:** "when am I meeting X", free/busy checks (2025-10-14).
- **Product help:** "How do I share a meeting?" (2025-08-21). The CLI and MCP also expose support-article search.
- **Write actions** (REL:ask-circleback-to-update-meetings, 2026-10-07). The assistant can:
  - create, edit, assign, complete and delete action items
  - create, edit and delete tags, and add or remove them on meetings
  - share meetings with people, teams, the workspace or a link
  - rename meetings and edit their notes
  - add private notes and tags to **upcoming** meetings (this needs a connected calendar)

  It works on the open meeting by default; for others, you name the meeting, date, people or tags. It **respects existing permissions**.
- **Email drafting and sending** (§4).
- **Connectors** (REL:ask-circleback-to-do-things-in-your-apps, 2026-07-28):
  - Open them from the plug icon next to the Ask input.
  - Built-in connectors: **Linear, Notion, Asana, HubSpot, PostHog, Attio, Ashby, Intercom**.
  - **Any MCP server can be added as a custom connector.**
  - Connectors are available on web, desktop, mobile and Slack, and are managed under Settings → Connected accounts.
  - Per-connector capabilities:
    - **Linear:** search, filter and read issues. Create and update issues (title, description, team, status, cycle, labels, assignee, project, milestone, priority, parent and duplicate links, due date, estimate, links, blocking and related relations). Comments on issues, documents, projects, initiatives and milestones. Projects and initiatives, milestones, cycles, documents, teams and users, statuses and labels (including creating labels), and Linear docs search. If Linear is already connected for automations, the same connection is reused. (2026-06-09 for the first version.)
    - **Notion:** search and fetch pages and databases. Query data sources, views and meeting-notes data. Create and update pages. Read and add comments. List teams and users. Check async operation status.
    - **Asana:** search; read, list (mine), create and update tasks; list, read and create projects; project status overviews; create status updates; comment on tasks; look up users and teams.
    - **HubSpot:** search and read CRM objects (contacts, companies, deals, tickets and others); create and update records; property definitions; owners; campaign contacts by engagement type; campaign analytics and asset metrics.
    - **PostHog:** projects, users, event and property schemas, warehouse schemas. Actions (create, list, update). Dashboards (create, update; tiles; run insights). Insights and arbitrary queries (trends, funnels, retention, paths, lifecycle, stickiness, web analytics). Activity. Feature flags (CRUD, dependencies, evaluation testing, blast radius). Scheduled flag changes. Docs search.
    - **Attio:** records (search, list, read, create, upsert, update); attributes and lists (add records to lists, update entries); comments; notes (create, semantic search, update); tasks; meetings and call recordings (search and read); emails (search and read); workspace members and teams; reports and **Particle SQL** queries.
    - **Ashby:** search and filter records; candidates (read, create); add candidate notes; move application stage and consider a candidate for a job; interviews (upcoming list, details, plans, submitted feedback); job pipelines; pending tasks; Ashby docs search.
    - **Intercom:** search contacts and conversations with structured filters (state, channel, assignment, timing, ratings, AI Agent fields); companies; Help Center articles (list, search, read, create, update title, summary, content, state and location).
  - Connector example: "Check PostHog to see who from this week's demos hasn't signed up yet."

### 9.3 Per-meeting assistant

- **Location:** the right panel's **Assistant** tab on any meeting. It started as the original "meeting assistant" (2023-04-03).
- **What it does:**
  - Answers questions about this meeting with **sources** that jump to the moment (2025-03-25).
  - Writing tasks, such as a follow-up email.
  - Update actions on this meeting (2026-10): add an action item, edit notes, find a decision and so on.
- **Chat history is saved per meeting** (2025-04-08). Clear it with ⌘⇧⌫ or Ctrl+Shift+Backspace.
- **Panel controls:** the panel can be resized and responses copied (2024-10-25).
- **Link viewers:** people viewing through a public link can use the meeting assistant and search, but can't edit (2024-01-04).

### 9.4 Assistant in Slack

Sources: DOC:assistant/use-assistant-in-slack; REL:ask-circleback-in-slack (2025-11-18); REL:do-more-with-circleback-in-slack (2026-08-03).

- **How to ask:** `@Circleback <question>` in any channel, or a DM to the app.
- **Follow-ups:** in threads. In channels you have to @mention again; in DMs no mention is needed.
- **Thread awareness:** asking in a thread makes Circleback read the **earlier thread messages, including images**. Answers come with sources.
- **Link unfurls:** sharing a Circleback meeting link shows a **meeting preview**. If channel members lack access, Circleback **prompts the sharer to share it with them**.
- **Agent panel:** Circleback can be opened from **Slack's agent panel**.
- **Scope:** it can update meetings and use connectors with your Circleback permissions.
- **Setup:** connect Slack under Settings → Connected accounts.

---

## 10. Automations (formerly "Workflows")

History: workflows launched 2024-02-15 and were renamed Automations on 2024-11-01. Sources: DOC:automations/*; REL entries.

### 10.1 Model

- **Structure:** an automation has a **name**, **conditions** ("when to run") and an ordered list of **actions** (steps). Actions can be chained: for example, generate an insight, then push that insight to Slack or a CRM.
- **Conditions** (REL:improved-automation-trigger-conditions, 2025-01-24, and later):
  - The automation holds several **condition groups**. **Within a group every condition must match (AND)**, and **if any group matches the automation runs (OR)**.
  - **Condition types:**
    - **Tags:** include or exclude.
    - **Participants:** include or exclude specific people.
    - **Meeting name:** is, is not, contains, does not contain.
    - **Invitee emails.**
    - **Invitee email domains:** for example "are all" (internal only) or "are not all" (external present) (2025-04-03).
    - **Number of invitees:** is, greater than, less than N (2026-03-22). Example: exactly 2 invitees plus your domain gives internal 1:1s.
  - **No conditions** (the "Run after every meeting" button) runs on every meeting.
  - **Tag-triggered runs:** the automation runs when a tag is applied, manually or automatically. Tagging an upcoming meeting therefore pre-arms its automations.
- **When automations run:** **after meeting processing**, whatever the capture method.
  - "By default, automations run automatically." You can **disable auto-run** and only trigger them manually.
  - Disabled automations still show up in a meeting's ⋯ → Automations list for manual runs (2024-05-24).
- **Manual and past runs** (DOC:run-automations-for-past-meetings; REL:rerun-automations, 2025-11-10):
  - From a meeting: ⋯ → **Automations** shows the relevant automations and run status, with Run or Rerun.
  - From an automation: ⋯ → **Meetings** lists all matching past meetings, whether the automation already ran on each, and lets you run it on selected ones (2025-04-22).
  - Bulk: select meetings on the Meetings page → **Run automation** (2025-09-25).
- **Duplicate an automation** (2025-04-30).
- **Sharing** (DOC:share-an-automation, enable-automations-for-your-workspace):
  - ⋯ → Share → pick the workspace, a team or people, with an access level.
  - **Share**: the recipient sees the automation and can run it manually or enable it themselves.
  - **Enable**: admin only, via "Enable this automation for people added". It **runs automatically on their future meetings**, and they get an email.
  - **Shared automations use the creator's connected accounts** for integrations, so recipients need no setup (2024-07-28).
  - Recipients only see insights for meetings they can access.
- **Admins** can run automations on **any meeting they have access to**, not just their own (2026-02-16).
- **Plan gating:**
  - Free gets "limited automations"; press reports say the Slack and Linear integrations are included.
  - Pro+ unlocks all integrations, including Notion, HubSpot, Salesforce, Attio and **webhooks**, plus **custom AI insights**.

### 10.2 Action catalogue

DOC:automations/getting-started-with-automations lists the current set, plus a few from the blog (2026-03-11).

| Action | Configuration and behavior |
|---|---|
| **Generate insights with AI** | See §10.3. |
| **Share meeting** | Share with specific people, a team or the whole workspace. Option: **"limit sharing to invited team members"** (2025-08-04). Shared meetings appear on recipients' Meetings page. This replaced the old "auto-share" team toggle (2024-09-30). |
| **Send webhook request** | Endpoint URL plus toggles for which outcomes to include (notes, action items, transcript, insights, recording, metadata). **Send a test request** using the most recent meeting. A **signing secret** is shown. Multiple webhook steps are allowed. See §12.4. |
| **Send notes to Slack** | Pick a channel; private channels require the Circleback app to be added. Toggle outcomes: notes, action items, insights, attendees, link. Posts are **formatted as a thread** (2024-05-29), with a parent message and details in replies. |
| **Update Notion** | Pick a **page** or **database**. If it isn't listed, add the Circleback connection in Notion via ⋯ → Connections. **Page:** choose which outcomes go into the created page content. **Database:** map each property (column) to an outcome (meeting fields, action items, insights, or *specific custom fields of an insight*). Mapping a multi-valued outcome (action items, multi-instance insights) to a **property** creates **one row per item**. Putting them in **page content** creates one row per meeting (2025-04-09). |
| **Create issues in Linear** | Team, plus the assigned/unassigned toggles (§8). |
| **Update HubSpot** | Update **contacts** or **deals**, matched by invitee email. Deals cover those associated with the contact **or its company** (2024-09-16). Writes a **logged Meeting engagement** (not a note, since 2025-02-25) with the selected outcomes. Action items become **tasks** assigned to HubSpot users. |
| **Update Salesforce** | Update **contacts** or **opportunities**, matched by invitee email, including opportunities of the contact's account. Creates an **Event** on the activity timeline (not a note, since 2025-05-21). |
| **Update Attio** | Update **people**, **companies** or **deals** (deals since 2025-10-14), matched by invitee email. Adds a **note** with the outcomes. Creates **tasks** for action items, assigned if the assignee is an Attio workspace member. |
| **Update Zoho contacts** | Adds a note with outcomes to each Zoho contact matched by invitee email (2024-03-28). |
| **Create items in monday.com** | Board plus group, one item per action item, assigned. Assigned and unassigned filters. |
| **Send notes to Zapier** | Connect Zapier and select outcomes. In Zapier: Trigger app Circleback, event **Automations**, then pick this automation step. **One step maps to exactly one Zap**; connecting a second Zap disconnects the first. |
| **Make** | Uses the webhook action. In Make, choose Circleback, then "Watch Webhook Request", and paste the hook URL into Circleback's webhook step. |
| **Send email / Draft email** (blog only, 2026-03-11) | Email the recap to all attendees, external only or internal only; "Draft" lets you review before sending. These aren't in the current help-center action list. |
| **Auto-tag action** (blog only, 2026-03-11) | Apply tags based on name patterns or attendee domains. |
| **Google Docs** (blog only, 2026-03-11) | "Select the shared folder; each meeting creates a new doc." Not in current docs or the integrations list, so treat it as unverified. |

**External-only pattern:** add a condition group where "invitee email domains **are not all**" equals your domain (DOC:run-automations-for-external-meetings).

### 10.3 AI insights (custom extraction)

Sources: DOC:automations/generate-insights-with-ai; REL:workflows (2024-02-15), REL:custom-fields-for-insights (2024-03-12), REL:number-of-insights-per-meeting (2024-04-05), REL:insight-templates (2025-01-03), REL:insights-tab (2025-02-25), REL:improved-insights-table (2024-06-12).

- **Definition:** an insight is an AI-generated output defined by a **name** and a **description**, which acts as the prompt. Two kinds are typical:
  - **Extraction:** questions asked, candidate qualifications, pain points, competitor mentions.
  - **Generation:** a follow-up email, memo or project proposal.
  - The description should say what to extract or generate and include an example or output template.
- **Templates:** a gallery of pre-built insight templates to start from, which you can use as-is or edit.
- **Custom fields:** optional typed fields that structure the output. For example, "Customer details" might have Company, Budget, Timeline and Pain points. Each field is filled separately, and numeric types exist (the example `"Number of employees": 120`).
- **Number of insights:** **Auto** (the AI decides how many instances to return, for example one per question asked) or a fixed count such as **One** (for example "How did this customer hear about us?").
- **Output shape** (webhook and API): `insights: { [insightName]: [ { insight: string | {field: value}, speaker: string|null, timestamp: number|null } ] }`. Each instance can be **attributed to a speaker and a timestamp**.
- **Where results show up:**
  - The meeting's **Insights** tab.
  - The global **Insights** page in the left nav: a table per insight across all meetings, with horizontally scrolling custom-field columns, expandable cells and **CSV download**.
- **Downstream use:** insights can feed Slack, Notion property mapping (per field), CRM notes, webhooks and Zapier.
- **Plan:** "Custom AI insights" requires Pro+.
- **Example recipes** (blog, 2026-03-11): deal summary to CRM; competitor mentions to Slack; discovery scorecard (questions, pain, budget, timeline, decision-maker, 1–5 score); customer feedback (feature requests, complaints, praise, quotes); health signals (churn phrases, Red/Yellow/Green); executive summary; interview debrief; cross-meeting themes via Zapier to Google Sheets.

---

## 11. Integrations matrix (current)

| Category | Integrations |
|---|---|
| Calendar | Google Calendar, Microsoft Outlook Calendar (multiple accounts, primary calendars) |
| Email (context, drafting, sending) | Gmail, Outlook Mail |
| Meeting platforms (bot) | Zoom (including registration webinars), Google Meet, Microsoft Teams, Webex; Slack huddles through a member account. GoTo is listed in the FAQ. |
| Meeting platforms (desktop) | Any app that uses the mic: Discord, RingCentral, WhatsApp and so on |
| Automation destinations (native) | Slack, Notion, Linear, HubSpot, Salesforce, Attio, Zoho CRM, monday.com, Zapier, Make (through webhook), generic webhooks. Blog also mentions Google Docs and Send/Draft email. |
| Assistant connectors (read and write) | Linear, Notion, Asana, HubSpot, PostHog, Attio, Ashby, Intercom, plus custom MCP servers |
| Identity | Google and Apple login (2025-03-20), email one-time code login (default), SSO (SAML or OIDC: Okta, Google Workspace, Entra ID, generic), SCIM (Entra, Okta, generic SCIM 2.0) |
| AI clients (through MCP) | Claude (directory connector), ChatGPT (app), Cursor (marketplace plugin), Claude Code, Codex, Raycast, any Streamable-HTTP MCP client |
| Importers | Fathom, Fireflies, Granola, Read AI, plus the API import endpoint |
| Other | Raycast extension; HubSpot marketplace listing; monday.com marketplace; Make app; Zapier app |
| External links on people and companies | API `externalLinks.type` ∈ Attio, HubSpot, Linear, Salesforce, Zoho, linkedin, facebook, instagram, twitter, website |

---

## 12. Developer platform

### 12.1 REST API

Released 2026-08-20; write access and import added 2026-09-09 and 2026-10-07. Source: https://circleback.ai/docs/api and `/docs/api/openapi.json` ("Circleback Public API" v1.0.0).

- **Base URL and auth:** `https://circleback.ai/api`. Send `Authorization: Bearer cb_<secret>`.
  - Keys are created under Settings → API keys and shown once.
  - Keys are **read-only** or **read and write**.
  - `GET /oauth/test` returns `{email, apiKeyName}`.
- **Format:** JSON throughout.
- **Pagination:** cursor-based through an RFC 8288 `Link: <...?cursor=...>; rel="next"` header. Cursors are opaque and tied to their filters.
- **Rate limits:** counted per account across all API keys and MCP and CLI OAuth tokens.
  - Free: 3 requests per second, 20 per minute. Pro and higher: 20 per second, 300 per minute.
  - Headers: `RateLimit-Limit`, `RateLimit-Remaining`, `RateLimit-Reset` (for the window closest to exhaustion).
  - A 429 response carries `Retry-After` and `{"error":"Ratelimit exceeded for the Free plan.","code":"RATE_LIMITED"}`.
- **Error envelope:** `{code, error, seatsToAdd?, issues:[{message,path}]}`.
- **Endpoints:**
  - **Meetings**
    - `GET /meetings`: list. Params: `ownership` (All, **Mine** (default) or Shared), `statuses[]`, `tagIds[]`, `attendeeProfileIds[]`, `cursor`.
    - `GET /search`: params `searchTerm`, `meetingIds[]`, `tagIds[]`, `attendeeProfileIds[]`, `cursor`. Searches names and content.
    - `GET /meeting/{id}`: returns the meeting object (below).
    - `GET /meeting/{id}/transcript`: returns `[{speaker, text, timestamp}]`.
    - `PUT /meeting/{id}`: body `{name?, notes? (markdown), privateNotes? (markdown), tagIds?}`. Each field **replaces** its current value.
    - `PUT /meeting/{id}/share`: body is an array of 1–50 targets, each one of:
      - `{email, access, sendEmail?}`
      - `{profileId, access, sendEmail?}`
      - `{teamId, access?, sendEmail?}`
      - `{workspaceId, access?, sendEmail?}`
      - `{linkAccess}`

      `access` ∈ Editor, Viewer, LimitedViewer, or `null` to remove. Partial failures are listed in `errors[{error,index}]`.
    - `POST /meetings` (import): body `{name, createdAt?, tags[{name,description}], attendees[{name,email,isCalendarEventOrganizer,isCalendarInvitee}], notes (md), transcript[{speaker, text, timestamp, language?} | {speaker, words[{text,startTimestamp,endTimestamp}]}], sharedWith[{email, access}]}`.
    - `DELETE /meeting/{id}`: owner only.
  - **Action items**
    - `GET /action-items`: params `assigneeType` (Me (default), NotMe, Profile, MyWorkspace, OutsideMyWorkspace, Unassigned, Anyone), `assigneeProfileId`, `assigneeTeamId`, `status` (defaults to incomplete), `attendeeProfileIds[]`, `tagIds[]`.
    - `POST /action-items`: body `{title, description?}`, owned by you and not linked to a meeting.
    - `PUT /action-item/{id}`: body `{title?, description?, assigneeProfileId? | null, status?}`.
    - `DELETE /action-item/{id}`.
  - **Calendar**
    - `GET /calendar/events`: params `startTimeLookbackHours`, `startTimeLookaheadHours`, `sortDirection`, `includeOfflineSingleAttendee`, `cursor`. Event fields are listed in §3.2.
    - `PUT /calendar/events/{calendarMeetingId}/meeting`: sets `privateNotes` and `tagIds` on the meeting for an upcoming event, **creating the meeting record if needed**.
  - **Companies**
    - `GET /companies?tagIds[]`: companies whose people attend your meetings.
    - `GET /company/{domain}`: `{name, avatarUrl (Clearbit logo), domain, externalLinks[], people[]}`.
  - **People**
    - `GET /people?domains[]&tagIds[]&limit`: ordered by most recent meeting.
    - `GET /person/{profileId}`: `{id, firstName, lastName, email, title, companyId, companyName, externalLinks[]}`.
  - **Tags**
    - `GET /tag`, `POST /tag` (409 on a duplicate name), `PUT /tag/{id}` (name and description), `DELETE /tag/{id}` (removes the tag from all meetings).
    - A hidden `DELETE /tag` removes a tag from specific meetings.
- **Meeting object:** `id, name, createdAt, updatedAt, duration (s), url (join URL), recordingUrl (signed; webhook says valid 24h), tags[{id,name,description}], icalUid, attendees[{profileId,name,title,companyName,email,isCalendarEventOrganizer,isCalendarInvitee}], notes (md), privateNotes, actionItems[], insights{}, linkAccess, calendarEvent{id, icalUid, description, platform: GoogleCalendar|MicrosoftOutlook, platformId}, sharedWith[{profileId|teamId, name, email?, access: Owner|Editor|Viewer|LimitedViewer}]`.
- **Developer experience:** the docs site has a "Try it" console for every endpoint, plus code samples in cURL, JS, Go, Python, Java, C# and Rust.

### 12.2 MCP server

Released 2026-01-13; ChatGPT app 2026-03-09; recordings 2026-05-31; write tools 2026-10-07. Source: DOC:platform/circleback-mcp.

- **Endpoint:** `https://circleback.ai/api/mcp`. Remote, authenticated MCP using **Streamable HTTP**, with OAuth and **dynamic client registration**.
- **Write access:** requested at connect time. Clients connected earlier must reconnect to get it.
- **Tools:**
  - `SearchMeetings`: keyword, date range, tags, attendee, domain.
  - `ReadMeetings`: notes, private notes, attendees, action items, insights, tags, duration and a **recording download link**.
  - `UpdateMeeting`
  - `UpdateCalendarEvent`: private notes and tags before the meeting starts.
  - `ShareMeeting`
  - `SearchTranscripts`: returns hits with timestamps.
  - `GetTranscriptsForMeetings`
  - `SearchActionItems`, `CreateActionItem`, `UpdateActionItem`, `DeleteActionItem`
  - `SearchCalendarEvents`: includes join status.
  - `SearchEmails`
  - `FindProfiles`: person profile plus interaction history.
  - `FindCompanies`
  - `ListTeams`, `ListTags`, `CreateTag`, `UpdateTag`, `DeleteTag`
  - `SearchSupportArticles`
- **Client setup:**
  - Claude: Settings → Connectors → Browse → Circleback.
  - ChatGPT: the app link.
  - Raycast: deep link, then `@circleback`.
  - Claude Code: `claude mcp add --transport http circleback https://circleback.ai/api/mcp`, then `/mcp`.
  - Cursor: marketplace plugin.
  - Codex: `codex mcp add circleback --url https://circleback.ai/api/mcp`, with `experimental_use_rmcp_client`.

### 12.3 CLI

Released 2026-03-18; write commands in v0.5.0 (2026-10). Source: DOC:platform/circleback-cli.

- **Install:** `npm install -g @circleback/cli`.
- **Auth:**
  - `cb auth login` uses browser OAuth.
  - `cb auth login --api-key` stores a key locally.
  - The env var `CIRCLEBACK_API_KEY` takes precedence for stateless use.
  - `cb auth logout`, `cb update`.
- **Commands:**
  - `cb meetings [search]`: flags `--last N`, `--from/--to`, tag, person, company.
  - `cb meetings read <ids...>`: up to 50 meetings.
  - `cb meetings update <id> --name ... --tags A,7`. Tags can be given by name or ID; the list replaces existing tags, and `""` clears them.
  - `cb meetings share <id> --email a,b --access Viewer --send-email | --link Viewer|none | --team 7 | --profile 42 --access none`.
  - `cb transcripts <search>`, `cb transcripts read <ids...>`.
  - `cb action-items [search] --status pending|done`; `create "<title>" --description --assignee <id|none>` (assigns to you by default); `update <id> --status DONE --assignee none`; `delete`.
  - `cb calendar [search] --from --to`; `cb calendar update <eventId> --private-notes "..."`.
  - `cb emails search "... participant:x from:y after:2026-01-01 before:..."`.
  - `cb people <names>`, `cb companies <terms>`, `cb teams`.
  - `cb tags [list|create <name> --color emerald|update <id> --name|delete <id>]`.
  - `cb support search "..."`.
- **Output:** `--json` on every command.

### 12.4 Webhooks

Source: DOC:automations/export-meeting-data-with-webhooks. Introduced 2023-08-15 as a setting, moved into automations on 2024-12-12, and got signing on 2024-04-17.

- **Request:** a POST with a JSON body per matching meeting:
  - `id` (open with `https://circleback.ai/meetings/${id}`)
  - `name`, `createdAt` (ISO), `duration` (s), `url` (join URL)
  - `recordingUrl` (valid 24h; only present when recordings are saved)
  - `tags[]` (names)
  - `icalUid`
  - `attendees[{name,email}]`
  - `notes` (Markdown)
  - `actionItems[{id,title,description,assignee{name,email}|null,status}]`
  - `transcript[{speaker,text,timestamp}]`
  - `insights{name:[{insight, speaker, timestamp}]}`
- **Field selection:** you choose which outcomes are included.
- **Signature:** `x-signature` header containing the hex HMAC-SHA256 of the raw body, keyed with the step's signing secret. Compare with timingSafeEqual.
- **Testing:** "Send test request" uses your latest meeting.
- **Backfill:** select all meetings and run the webhook automation on them.

---

## 13. Emails, sharing and permissions

### 13.1 Post-meeting emails

Sources: DOC:account-and-setup/email-preferences; REL:include-recording-and-transcript-access-in-invitee-emails (2024-12-05).

- **Recipients** (Settings → Emails):
  - **Myself:** a notes email after every meeting.
  - **Everyone invited:** an automatic notes email to every **calendar invitee**. Only calendar invitees get the automatic email.
- **Content:** notes and action items are always included, with checkboxes for marking items done (§8). An optional toggle adds a **link to the recording and transcript**, which auto-creates a shareable link.
- **Manual send:** Share → **Email** tab. You can email notes to chosen people even when automatic emails are off (2024-05-03).
- **Share emails:** when a meeting is shared with someone, they get an email with the notes and a link (`sendEmail` in the API).
- **Weekly roundup:** an opt-in weekly email summarizing the most important details and themes from your meetings and meetings shared with you over the last 7 days (2025-09-23).
- **Other emails:** comment notifications, automation-enabled notices, import-complete, export-ready, admin notices for auto-added users and join requests, data-retention changes, and referral credits.
- **Referral attribution:** notes emails carry referral attribution (a free month).
- **Admin control:** email preferences can be set as a default or enforced by admins.
- **Deliverability:** a hard bounce suppresses the address, and logging in by email clears the suppression. Allowlist circleback.ai.

### 13.2 Sharing model

Sources: DOC:meetings/share-a-meeting; DOC:account-and-setup/sharing-preferences; REL:share-meetings-with-anyone (2026-07-16); REL:improved-share-modal (2026-03-10).

- **Opening the share modal:** right-click a meeting, or open it and click **Share**.
- **Share modal:**
  - Search for workspace members, attendees or teams by name or email, or enter **any email**, including people outside the workspace who weren't invited.
  - Pick an access level and click **Share**.
  - The modal lists everyone with access, including people who got the notes by email.
- **Access levels**, for specific people and for link sharing:

  | Level | Can do |
  |---|---|
  | **Limited viewer** | Notes and action items only; can mark their own action items done. |
  | **Viewer** | Adds the transcript and recording. |
  | **Editor** | Can edit notes, action items and transcript (speaker names). Editing through a link requires login. |
  | **Owner** | Implicit; can delete, run automations and share. |

- **Link access:** "anyone with the link" at Limited viewer, Viewer or Editor level, or restricted. Viewing through a link **doesn't require login**. Link viewers can also use the meeting assistant and search (2024). The control sits at the bottom of the modal next to **Copy link**.
- **Unsharing:** set a person's access to **None**.
- **Sharing defaults** (Settings → Sharing):
  - Default access for **workspace invitees**.
  - Default access for **external invitees**.
  - Meetings are shared automatically with calendar invitees after the meeting.
  - Admins can set defaults or enforce them.
- **Auto-share by automation** (§10.2) to people, teams or the workspace, optionally limited to invited members.
- **Admin sharing:** admins with editor access can share meetings **owned by others** in the workspace (2026-07-27).
- **Request access** and **claim ownership** (§7.8).
- **Restrictions for external users:** they can't download recordings or see comments.

---

## 14. Organization and navigation

- **Home:** upcoming meetings with join toggles, tagging, details panel and "Review last meeting".
- **Meetings page:**
  - Shows invitee avatars, colour-coded tags, and the full date on hover (2025-01-13).
  - Filter by ownership: All, Mine or Shared with me (2024-06-18).
  - Filter by **participants**, **companies** (2025-12-05), **tags** and **date** (a single day or range, 2026-10-06).
  - Filters persist. `F` opens keyboard filter search.
  - **Views** (2026-03-19): save a filter set as a named tab with the `+` next to the tabs.
  - **Bulk actions** (2025-08-24 and 2025-09-25):
    - Select by checkbox or avatar, Shift-click for a range, ⌘/Ctrl+A for *all matching the filters, including meetings not yet loaded*, or click-drag. Esc clears the selection.
    - The bottom action bar or ⌘K offers: add or remove tags, share, remove access, run automation, delete (⌘⌫).
    - Only fully processed meetings can be selected.
  - **Right-click context menu:** share, run automation, tag, create a new tag (2024-12-09), open calendar event, email invitees, delete. It works even while a meeting is processing.
- **Tags** (DOC:meetings/tag-meetings; manage tags shipped 2026-01-29):
  - Add from Home (on upcoming meetings), from the Meetings page, or inside a meeting.
  - Settings → Tags: create, rename, recolor, **describe** and delete.
  - **Automatic tagging** (toggle in Settings → Tags; first shipped 2024-01-23):
    - After processing, the AI applies relevant **existing** tags. It never creates new ones.
    - It decides using tag descriptions and **recent meetings that carry each tag**, so it learns from consistent manual tagging.
    - **If you tag a meeting manually before processing finishes, auto-tagging is skipped.**
- **People and company Spaces** (REL:people-and-companies, 2026-04-30; summaries 2026-08-03):
  - Every person and company gets a page, reachable from **Spaces** in the sidebar, from search, or by clicking a name in a meeting.
  - Each page shows **pending action items**, recent **meetings and emails**, **upcoming calendar events**, attendees, and deep links to the matching record in Attio, HubSpot, Salesforce, Zoho or Linear.
  - An AI **summary** of "who they are, what's been discussed, and where things stand" draws on **all meetings and emails you have access to, including teammates'**, and refreshes automatically.
  - You can **ask questions scoped to the person or company** from the page.
  - Company logos come from Clearbit. Enrichment includes job title and company (the details panel shows role and company website).
- **There are no folders.** Organization relies on tags, filters, views, teams and spaces.
- **Keyboard and UX:**
  - `/` or ⌘K: search and navigate.
  - `[`: toggle the sidebar. `]`: toggle the right panel (2026-02-20).
  - `F`: filters. ⌘F: transcript search.
  - Dark mode on web and desktop (Light, Dark or System, 2026-02-12).
  - Accessibility pass (2026-02-18).
  - New design (2026-01-27).
  - Profile images (2026-07-17).
  - **Multiple accounts** with a switcher (2026-09-08).
- **"Circling back on <year>":** a yearly in-app recap of your meetings, with counts, time spent and who you met with most. Available for users with more than 5 meetings (2024-12-03 and 2025-12-08).

---

## 15. Workspace, teams and admin

Sources: DOC:workspace-and-teams/*; REL:organize-your-workspace-into-teams (2025-10-31).

- **Workspace:** Settings → General (Workspace) → Create workspace. The creator becomes **admin and billing owner**, and there's no member limit.
  - The workspace has a name and an **image or logo**, which shows wherever the workspace or company appears (2026-09-17).
- **Roles:**
  - **Admin:** manages settings, members and automations.
  - **Member:** a paid seat; uses Circleback for their own meetings.
  - **Viewer:** a **free** seat; can only access what is shared and can't record. On Business, free viewers are "unlimited".
  - **Billing owner:** a separate flag, the only person who can manage payment.
  - Actions under Settings → Members → ⋯: make admin, member or viewer; make billing owner; deactivate or reactivate.
- **Adding people:**
  - By email; people without an account get invited.
  - **Automatically add new users**: anyone who signs up with a matching domain becomes a member, and admins get an email.
  - **Ask to join**: same-domain signups can request to join, and admins approve or decline (2025-07-28).
  - **Multiple domains**: add a domain with + next to Domains, verified by an email code (2026-05-29).
- **Teams** (sub-groups):
  - Settings → Teams → Create team → Add members. A member can belong to several teams.
  - Teams are used for sharing meetings and enabling automations.
  - Limits by plan: Free 2, Pro 5, Business unlimited.
- **Deactivation:** removes access, signs the user out on all devices, keeps their shared meetings accessible to others, and frees the seat at the end of the cycle. SCIM can deactivate automatically.
- **Default and enforced settings** (Settings → General):
  - A **default** applies to anyone who hasn't set it themselves. The **lock icon** enforces it for all members.
  - Settings covered: save recordings; automatic tagging; vocabulary; meeting joining preferences; sharing defaults; email sending preferences; meeting participant name and image; data retention.
- **Data retention** (Settings → General → Data retention):
  - Three sliders: **Recording**, **Transcript** and **All meeting data** (notes, action items, insights and so on).
  - The transcript can't be kept for a shorter period than the recording.
  - Applies to all members' meetings, including unshared ones. Expired data is **permanently deleted**.
  - All members are emailed when it changes, and changes take effect within 24 hours (2024-09-26).
- **Usage dashboard** (Settings → Usage, 2025-11-30):
  - Metrics over selectable periods: number of meetings, total meeting time, unique people met, automations run.
  - An internal vs external breakdown, and a **leaderboard** across the workspace and teams.
  - A GitHub-style **meeting activity heatmap** covering the year (2026-07-01).
  - Admins see the workspace, teams and individuals; others see only themselves.
- **SSO and SCIM** (Enterprise, 2026-09-16; DOC:security/single-sign-on/*):
  - Settings → General → Single sign-on → Connect → choose SAML or OIDC, then enter the IdP details.
  - Guides exist for Google Workspace (SAML), Entra ID, Okta, generic SAML 2.0 and generic OIDC.
  - **One IdP per workspace.**
  - **Enforce SSO** signs out members who didn't log in with SSO and blocks every other login method. Test in a private window first, keeping the admin session open.
  - **Directory sync (SCIM):** Microsoft Entra ID, Okta or Generic SCIM 2.0. Circleback shows a Directory endpoint and token.
  - SCIM only manages members whose domain matches a verified domain. It can't deactivate the billing owner or the last admin.
  - Default login without SSO: an emailed one-time code, which inherits your email provider's SSO and MFA. Google and Apple login are also available.
- **HIPAA** (Enterprise; DOC:security/enable-hipaa-compliance, 2026-09-01):
  - Settings → General → HIPAA compliance → turn on Business Associate Agreement, then review and accept. It takes effect immediately.
  - The standard BAA (circleback.ai/baa) is used, with no custom BAAs.
  - Not for 42 CFR Part 2 records.
  - Coverage ends if the toggle is turned off or the plan leaves Enterprise.
- **Billing ownership transfer** and **consolidated billing** (§1).

---

## 16. Privacy, security and compliance

Sources: DOC:security/security; https://circleback.ai/dpa; App Store privacy section.

- **Certifications:** **SOC 2 Type II** and **EU-U.S. Data Privacy Framework**, including the UK Extension; the ICO is the dispute body. Circleback supports **GDPR** as a processor under its DPA, which is governed by Irish law. **HIPAA** compliance is available on Enterprise.
- **Encryption:** at rest and in transit.
- **No training on customer data** (repeated on the homepage FAQ, docs and HN).
- **Security portal:** https://security.circleback.ai, a trust center with controls, the **subprocessor list** and an **external pentest report**. Subprocessor changes are announced 14 days ahead, and customers have 7 days to object.
- **Data residency:** primary processing happens in the US. There's no EU hosting option; the compare page lists this as a tradeoff.
- **Retention:**
  - Plan-based recording storage (§3.11).
  - Admin retention sliders (§15).
  - Without configured retention, data persists while the account is active.
  - Recently deleted meetings are kept 30 days.
  - Account deletion is immediate and irreversible.
  - Self-serve full export (§5).
- **Email:** accessed on demand and not stored.
- **Recording consent:**
  - Bot mode posts a chat message that can't be disabled.
  - Desktop mode gives **no notice to other participants**. Circleback's own blog says disclosure is the user's responsibility (https://circleback.ai/blog/bot-vs-desktop-recording, 2026-02-16; https://circleback.ai/blog/recording-consent-for-ai-meeting-notes, 2026-02-24).
  - Phone-call recording relies on the OS announcement.
  - There is no documented in-product consent prompt or participant opt-out flow for desktop recordings.
  - DPA: the customer is responsible for consents and notices to meeting participants. Participants who want to exercise privacy rights must go through the organizing customer.
- **App privacy labels** (iOS): data linked to you includes contact info (email, name), user content (audio) and usage data, used for analytics and app functionality. Google Play says no data is shared with third parties.

---

## 17. Launches in 2025–2026 (highlights; full list in Appendix A)

**2026**

| Date | Feature |
|---|---|
| Oct 7 | Assistant can **update** meetings, action items, tags, sharing and private notes; **write access** in MCP, CLI and API |
| Oct 6 | Date filter on Meetings |
| Sep 17 | Workspace image |
| Sep 16 | **SSO (SAML/OIDC) and SCIM**, Enforce SSO |
| Sep 14 | Invoice billing and seats by invoice |
| Sep 9 | API meeting **import** |
| Sep 8 | Multi-account switcher |
| Sep 1 | Self-serve **HIPAA BAA** toggle |
| Aug 31 | **New plans: Free, Pro, Business, Enterprise** (permanent free tier) |
| Aug 28 | **Import from Fathom, Fireflies, Granola and Read AI** |
| Aug 20 | **Public REST API** |
| Aug 10 | Request access to meetings |
| Aug 3 | **Person and company AI summaries**; Slack: thread-aware answers, link previews, agent panel |
| Jul 31 | **Draft and send emails** from Circleback |
| Jul 28 | **Assistant connectors**: Notion, Linear, Asana, Attio, HubSpot, PostHog, Ashby, Intercom, plus custom MCP |
| Jul 27 | Admins can share others' meetings |
| Jul 17 | Profile images |
| Jul 16 | Share with **anyone**; access levels Limited viewer, Viewer and Editor; sharing defaults |
| Jul 8 | Import call recordings and voice memos on mobile (share sheet) |
| Jul 2 | Recently deleted (30-day restore) |
| Jul 1 | Meeting activity heatmap |
| Jun 29 | **Apple Watch app** |
| Jun 17 | Ignored apps for meeting detection |
| Jun 9 | Ask Circleback about Linear |
| May 31 | Recordings available through MCP and CLI |
| May 29 | Always-visible record panel; multiple workspace domains |
| May 28 | **Notes capture what's shared on screen** |
| May 25 | In-context transcript search, including by speaker |
| May 19 | Filter action items by company |
| Apr 30 | **People and Companies spaces** |
| Apr 8 | Automatic mic switching |
| Apr 3 | Bulk actions on action items |
| Mar 22 | Number-of-invitees condition |
| Mar 19 | **Saved Views** |
| Mar 18 | **CLI** |
| Mar 11 | Mobile dark mode |
| Mar 10 | Improved share modal |
| Mar 9 | **ChatGPT app**; switch mic or window mid-recording |
| Mar 5 | Toggle for recording meeting video on desktop |
| Mar 3 | Auto-end on mic release |
| Mar 2 | Change calendar event mid-recording |
| Feb 26 | Quick navigation (⌘K) |
| Feb 20 | Collapsible sidebar |
| Feb 18 | Accessibility |
| Feb 16 | Admins run automations on shared meetings |
| Feb 12 | Dark mode |
| Feb 11 | Hide solo meetings in the menu bar |
| Jan 30 | Recording retention raised to 2 years |
| Jan 29 | Manage tags |
| Jan 27 | **New design** |
| Jan 13 | **MCP server** |
| Jan 7 | Keyboard filtering |

**2025**

| Date | Feature |
|---|---|
| Dec 23 | Claim ownership |
| Dec 11 | **iOS widgets and Action button** |
| Dec 5 | Filter meetings by company |
| Nov 30 | **Usage dashboard** |
| Nov 18 | **Ask Circleback in Slack** |
| Nov 10 | Rerun automations |
| Oct 31 | **Workspaces with Teams** |
| Oct 28 | **Connect email**; per-segment speaker rename |
| Oct 22 | Synced transcript on mobile |
| Oct 14 | Attio deals; assistant can answer calendar questions |
| Sep 30 | Private notes on mobile |
| Sep 25 | Bulk share and run automations |
| Sep 23 | **Weekly roundup** |
| Sep 16 | Desktop recording notifications |
| Sep 9 | Raycast extension |
| Aug 24 | Bulk select meetings |
| Aug 21 | Assistant answers product questions |
| Aug 14 | **Private notes** |
| Aug 13 | Mobile share and mobile Assistant |
| Aug 6 | **Hybrid-room speaker separation** |
| Aug 4 | Auto-share only with invited team members |
| Jul 29 | Recurring-meeting join choice |
| Jul 28 | Ask to join team |
| Jul 21 | Auto-start on mic use |
| Jul 17 | **Circleback Assistant** (cross-meeting, agentic, cited) |
| Jul 2 | Filter action items by tag |
| Jun 30 | People and companies in search |
| Jun 17 | Email-draft editor |
| Jun 16 | Mark action items done from email |
| Jun 6 | Event descriptions and email invitees |
| Jun 5 | Live Activity controls; record panel dark mode |
| May 31 | **Clips** |
| May 30 | Mobile search |
| May 28 | @tags in search; mobile editing |
| May 21 | Salesforce Events |
| Apr 30 | Duplicate automation; HubSpot tasks |
| Apr 23 | New notes editor; auto-add users |
| Apr 22 | Meetings relevant to an automation |
| Apr 17 | Mark all mine done |
| Apr 14 | Linear and monday.com assignment filters |
| Apr 9 | Notion database row behavior |
| Apr 8 | Per-meeting chat history |
| Apr 7 | **Android** |
| Apr 4 | Enable automations for team |
| Apr 3 | Invitee email and domain conditions |
| Mar 26 | **Team default and enforced settings** |
| Mar 25 | Sources for assistant answers |
| Mar 24 | Live Activity |
| Mar 20 | Google and Apple login |
| Mar 13 | Desktop auto start and end (calendar-based) |
| Mar 6 | Record panel |
| Mar 5 | Associate linked meetings with events |
| Feb 27 | Offline mobile recording; background playback |
| Feb 25 | HubSpot logs meetings |
| Feb 21 | Instant recording start |
| Feb 19 | **iOS app** |
| Feb 6 | Associate imports with events |
| Jan 28 | Streaming assistant |
| Jan 24 | **Condition groups (AND/OR)** |
| Jan 15 | Open calendar event |
| Jan 13 | Meetings page refresh |
| Jan 6 | **Zapier** |
| Jan 3 | **Insight templates** |

---

## 18. What users love

Every review site below has an angle (competitor or affiliate), so treat the sentiment as directional.

- **Notes quality and structure:** "structured, readable", useful to people who missed the meeting. MeetingPick (2026-08-10) gives it 5 stars on note quality. Founders often call it "the best" (startups page). App Store: "so good, it's almost funny" (2025-05).
- **Action items:** automatic assignment, 1–2 sentence context, and **one universal task list across all meetings**. Garry Tan cites "action and todos tracking"; Behrouz Hariri (Nora) named the universal task list as "the feature that made me switch"; on G2, "action items with a 1-2 sentence context is fantastic."
- **Automations:** a native no-code builder (no Zapier needed for most flows), custom insights, CRM and Linear routing. Mux's founder: "Love the webhook feature."
- **Flexible capture:** bot or no bot, mobile in-person recording, Watch, imports. Reddit users, as summarized by switchmytool (2026-07-09), like that there's "no awkward bot."
- **Cross-meeting search and email context:** answers with citations; Gmail context. Trustpilot 5★, 2025-11.
- **Simplicity and speed of shipping:** "Dead simple product that just works" (Metronome CTO). They ship very frequently: 219 changelog entries in about 3.5 years, and dozens of App Store builds a month.
- **Developer and agent access:** MCP, CLI and API. MeetingPick notes Otter and Tactiq lack MCP.

## 19. Complaints and missing features (by recency)

**Reliability**
- Trustpilot, 2026-09-01, 2★: "Keeps crashing or simply failing to take notes"; they only find out after the meeting; the team is "adding features instead of fixing core."
- Trustpilot, 2026-06-17, 1★: "best AI notetaker... WHEN IT WORKS"; repeated crashes.
- Google Play: a lost in-person recording that support couldn't recover.
- G2 and tl;dv mention iPhone app bugs.

**Support**
- A chatbot-first support widget whose canned replies sometimes blocked reaching a human (Trustpilot, 2026-06-17).
- Slow billing support; some users reported charges continuing after cancellation emails (HappyScribe, 2026-03-06, citing Capterra).

**Mobile app UX** (App Store review "Awful app, fine desktop", 2026-08-19)
- You can't start a new meeting while another is uploading or transcribing.
- You can't leave the chat view.
- You can't ask chat about a specific meeting without naming it.
- Some key functions are desktop-only (Google Play).

**Onboarding**
- There's no setup guide or sample output, and the UX "feels incomplete" (Trustpilot, 2026-02-19).
- **Recurring meetings needed per-occurrence toggling** in that user's experience. The 2025-07-29 "apply to all occurrences" modal partly addresses this.

**Speaker identification**
- Mislabels in calls with 4 or more participants, and in large in-person groups (Dupple, 2026).
- A tl;dv test (2026-01) found mixed-up speakers and a missed first 39 seconds.
- A Product Hunt user wanted in-room voice separation; this shipped 2025-08-06.

**Pricing** (mostly before 2026-08-31)
- There was no free tier, and the trial required a card (complaints from tl;dv, MeetingPick and Reddit, via switchmytool).
- An App Store review (2025-10-19) called it "Deceptive advertising": the app is free but needs a paid account.
- The free plan launched on 2026-08-31 largely resolves this, though Free keeps only 30 days of history.

**Speed:** notes take about 5 minutes, compared with Granola's near-instant output (Dupple).

**Missing or weak features reviewers cite**
- No editable notes template or format control. Verbosity is not self-serve; HN commenters asked for this in 2024.
- No inline citations inside the notes. An HN user asked in 2024; citations exist in the assistant but not inside the generated notes.
- No conversation analytics dashboards (talk ratios, topic trends) or sales coaching or forecasting. The usage dashboard covers volume only. Circleback itself concedes Gong, Read AI and Fathom go further here.
- No native Asana or Jira automation actions; those go through Zapier or Make. The Asana *assistant connector* exists.
- No due dates or priorities on action items, and no two-way task sync.
- No folders.
- No live, real-time transcript or notes during the meeting (not documented).
- Desktop recording gives other participants no consent notice.
- US-only data residency.
- Small team (about 8 people) with a short track record.
- Older reviews say "no REST API, only MCP" (a GitHub issue on onyx). The API shipped 2026-08-20, so this is fixed.

---

## 20. Re-implementation notes: key behaviors to replicate

1. **Capture orchestration:**
   - Calendar sync with per-event `willRecord` computed from the global rules (internal/external × organizer × RSVP) plus per-event or per-series override.
   - Bot dispatch, via a Recall.ai-like vendor, with de-duplication: one bot per meeting URL across users, and the chat message names everyone it represents.
   - Desktop agent: detect mic use per app (with an ignore list), prompt with an auto-start countdown, stop when the mic is released, fall back when the bot fails, and stream fMP4 segments.
2. **Meeting record unification:** one meeting per real meeting within a workspace, with multiple owners.
3. **Post-processing pipeline:**
   - Transcribe, diarize, then match voiceprints and calendar names.
   - Pull email context and screen-share OCR or vision.
   - Generate notes (Overview plus topic sections in Markdown), action items with assignee resolution to attendee profiles, and auto-tags (from tag descriptions plus few-shot examples from recently tagged meetings, existing tags only, skipped if manually tagged).
   - Then evaluate automations (condition groups) and run the action chain.
   - Then send emails to invitees per settings, and apply default sharing to invitees by internal/external level.
4. **Insights engine:** user-defined prompt, optional typed fields, cardinality (auto or N), and output instances carrying `speaker` and `timestamp`. Results feed a cross-meeting table with CSV export and Notion property mapping.
5. **Permissions:** Owner, Editor, Viewer and Limited viewer per principal (person, email, team, workspace, link). Private notes are per user. Comments are workspace-only. Downloads are blocked for external users. Admins can override sharing.
6. **Assistant:** an agent with retrieval tools over meetings, transcripts, emails, calendar, people and companies, plus write tools mirroring the API, plus third-party MCP connectors. Its surfaces are the web Ask page, the per-meeting panel, mobile, Slack (thread-aware, with images), and MCP for external AI clients.
7. **Entities:** profiles (people) and companies keyed by email and domain, with rolling AI summaries and external CRM links.

---

## 21. Primary source URLs

- Home: https://circleback.ai. Pricing: https://circleback.ai/pricing (also `/pricing.md` and `/pricing.json`). Releases: https://circleback.ai/releases (219 pages listed in https://circleback.ai/sitemap.xml).
- Help center: https://circleback.ai/docs/support. Full dump: https://circleback.ai/docs/llms-full.txt. Product guide: https://circleback.ai/llms.txt.
- API: https://circleback.ai/docs/api and https://circleback.ai/docs/api/openapi.json. MCP: https://circleback.ai/docs/mcp. CLI: https://circleback.ai/docs/cli (https://www.npmjs.com/package/@circleback/cli). Webhook: https://circleback.ai/docs/webhook.
- Desktop: https://circleback.ai/desktop. Mobile: https://circleback.ai/mobile. Startups: https://circleback.ai/startups. Affiliates: https://circleback.ai/affiliates. DPA: https://circleback.ai/dpa. BAA: https://circleback.ai/baa. Security: https://security.circleback.ai.
- Example shared meeting (notes format): https://circleback.ai/view/m9vtzdx5fpzle2iypnm.
- Compare pages, Circleback-authored: https://circleback.ai/compare/granola-vs-circleback (plus `.md`), and the fathom-, fireflies-ai-, otter-ai-, read-ai-, tldv-, gong- and krisp-vs-circleback pages. Index: https://circleback.ai/compare/llms.txt.
- Blog:
  - https://circleback.ai/blog/circleback-automation-setup-guide (2026-03-11)
  - https://circleback.ai/blog/circleback-automations-you-should-build (2026-03-11)
  - https://circleback.ai/blog/how-we-rebuilt-our-electron-recording-engine-in-swift (2026-04-16)
  - https://circleback.ai/blog/bot-vs-desktop-recording (2026-02-16)
  - https://circleback.ai/blog/recording-consent-for-ai-meeting-notes (2026-02-24)
  - https://circleback.ai/blog/best-ai-meeting-assistants (2026-03-11)
  - https://circleback.ai/blog/use-automations-to-identify-insights-in-meetings (2024-09-06)
  - https://circleback.ai/blog/seed-funding (2024-11-26)
- App stores: https://apps.apple.com/us/app/circleback-ai-meeting-notes/id6738971048 and https://play.google.com/store/apps/details?id=ai.circleback.app.
- Launch HN (2024-03-06): https://news.ycombinator.com/item?id=39618640. Product Hunt (launches 2023-03-14, 2024-02-20, 2025-04-25; 4.9★ from 9 reviews): https://www.producthunt.com/products/circleback-3.
- Press: TechCrunch 2024-11-26 https://techcrunch.com/2024/11/26/yc-backed-circleback-is-out-to-become-the-best-meeting-notetaker; free-plan coverage 2026-08-31 https://runtimewire.com/article/circleback-free-tier-meeting-notes-marketing.
- Reviews:
  - https://meetingpick.com/reviews/circleback-review-2026/ (2026-08-10)
  - https://www.switchmytool.com/blog/circleback-ai-review (2026-07-09)
  - https://dupple.com/reviews/circleback (2026)
  - https://tldv.io/blog/circleback-alternatives/ (2026-01-26)
  - https://www.happyscribe.com/blog/circleback-alternatives-for-ai-meeting-notes (2026-03-06)
  - https://www.trustpilot.com/review/circleback.ai (2.9 from 4 reviews)
  - https://www.g2.com/products/circleback-ai-inc-circleback/reviews (3.6 from about 6 reviews; blocked to the fetcher)
- Reddit couldn't be reached directly (blocked to the crawler). Reddit sentiment above comes secondhand from switchmytool and similar summaries.

## Appendix A — Complete release log (all 219 entries from circleback.ai/releases/*, newest first)

Format: date — title — first sentence (URL path under https://circleback.ai).

- **October 7, 2026** — Update Circleback with MCP, CLI, and API — You can now make changes in Circleback through MCP, the CLI, and the API. (`/releases/take-action-from-your-other-apps`)
- **October 7, 2026** — Ask Circleback to update meetings — You can now ask Circleback to update meetings, action items, and tags for you. (`/releases/ask-circleback-to-update-meetings`)
- **October 6, 2026** — Filter meetings by date — You can now filter meetings by date. (`/releases/filter-meetings-by-date`)
- **September 17, 2026** — Set your workspace image — Admins can now update the workspace image. (`/releases/set-your-workspace-image`)
- **September 16, 2026** — Single sign-on — You can now connect your identity provider to Circleback so workspace members can log in with SSO. (`/releases/single-sign-on`)
- **September 14, 2026** — Switch to invoice billing — You can now pay for Circleback by invoice instead of automatic payments. (`/releases/switch-to-invoice-billing`)
- **September 9, 2026** — Import meetings with the API — You can now import meetings into Circleback with the API. (`/releases/import-meetings-with-the-api`)
- **September 8, 2026** — Switch between accounts — You can now add multiple Circleback accounts and switch between them. (`/releases/switch-between-accounts`)
- **September 1, 2026** — Enable HIPAA compliance for your workspace — Admins can now enable HIPAA compliance for their workspace directly from settings. (`/releases/enable-hipaa-compliance-for-your-workspace`)
- **August 31, 2026** — New plans — Today we're launching a new lineup of Circleback plans: Free, Pro, Business, and Enterprise. (`/releases/new-plans`)
- **August 28, 2026** — Import meetings from other apps — If you have meetings in another app, you can now bring them into Circleback. (`/releases/import-meetings-from-other-apps`)
- **August 20, 2026** — Circleback API — Today we're releasing the Circleback API. (`/releases/circleback-api`)
- **August 10, 2026** — Request access to meetings — You can now request access to a meeting that hasn't been shared with you. (`/releases/request-access-to-meetings`)
- **August 3, 2026** — Person and company summaries — Every person and company now has a summary, a high-level view of what's going on with them. (`/releases/person-and-company-summaries`)
- **August 3, 2026** — Do more with Circleback in Slack — We've released several updates to our Slack integration: you can now ask about anything in a thread, preview shared meetings, and open Circleback from Slack's agent panel. (`/releases/do-more-with-circleback-in-slack`)
- **July 31, 2026** — Draft and send emails — You can now draft and send emails from Circleback. (`/releases/draft-and-send-emails`)
- **July 28, 2026** — Ask Circleback to do things in your apps — You can now connect Circleback to the apps you already use: Notion, Linear, Asana, Attio, HubSpot, PostHog, Ashby, Intercom, and more. (`/releases/ask-circleback-to-do-things-in-your-apps`)
- **July 27, 2026** — Workspace admins can share meetings — Workspace admins can now share meetings owned by someone in their workspace when they have editor access, not just meetings they own. (`/releases/workspace-admins-can-share-meetings`)
- **July 17, 2026** — Set your profile image — You can now set a profile image that shows up everywhere you appear in Circleback, including meetings, comments, and action items. (`/releases/set-your-profile-image`)
- **July 16, 2026** — Share meetings with anyone — We've made several updates to sharing that improve the whole experience: you can now share meetings with anyone, choose how much access people get, and set defaults for how meetings are shared with invitees. (`/releases/share-meetings-with-anyone`)
- **July 8, 2026** — Import call recordings on mobile — You can now import recordings to Circleback directly from your phone. (`/releases/import-call-recordings-on-mobile`)
- **July 2, 2026** — Restore deleted meetings — Deleting a meeting is no longer permanent. (`/releases/restore-deleted-meetings`)
- **July 1, 2026** — Meeting activity graph — You can now see your meeting activity over time as a graph in the usage dashboard. (`/releases/meeting-activity-graph`)
- **June 29, 2026** — Apple Watch app — Circleback is now on Apple Watch. (`/releases/apple-watch-app`)
- **June 17, 2026** — Ignored apps for meeting detection — You can now add apps that Circleback ignores when detecting meetings. (`/releases/ignore-specific-apps-when-detecting-meetings`)
- **June 9, 2026** — Ask Circleback about Linear — You can now ask Circleback about Linear and update it from the assistant. (`/releases/ask-circleback-about-linear`)
- **May 31, 2026** — Access recordings from MCP and CLI — You can now access meeting recordings through the Circleback MCP and CLI. (`/releases/access-recordings-from-mcp-and-cli`)
- **May 29, 2026** — Always-visible record panel — You'll now see a minimal panel the whole time you're recording on desktop, so you always know when Circleback is capturing your meeting. (`/releases/always-visible-record-panel`)
- **May 29, 2026** — Add multiple domains to your workspace — A workspace can now have more than one domain. (`/releases/add-multiple-domains-to-your-workspace`)
- **May 28, 2026** — Notes capture what's shared on screen — Circleback now captures details from what's shared on screen during your meetings. (`/releases/notes-capture-whats-shared-on-screen`)
- **May 25, 2026** — Search transcripts in context — You can now search a meeting transcript and see every result highlighted alongside the surrounding conversation. (`/releases/search-transcripts-in-context`)
- **May 19, 2026** — Filter action items by company — You can now filter action items by company. (`/releases/filter-action-items-by-company`)
- **April 30, 2026** — People and companies — Every person and company you meet with now has a dedicated space in Circleback. (`/releases/people-and-companies`)
- **April 8, 2026** — Automatic microphone switching — When you change microphones mid-meeting, the Circleback desktop app now automatically switches to recording with the new device without missing a word. (`/releases/automatic-microphone-switching`)
- **April 3, 2026** — Select and take actions on multiple action items — You can now select and take actions on multiple action items. (`/releases/select-and-take-actions-on-multiple-action-items`)
- **March 22, 2026** — Number of invitees automation condition — You can now run automations based on how many people are invited to a meeting. (`/releases/number-of-invitees-automation-condition`)
- **March 19, 2026** — Views — You can now save a set of filters on meetings and action items as a view and switch between views with one click. (`/releases/views`)
- **March 18, 2026** — Circleback CLI — You can now search and access your meetings, emails, calendar events, and more from your terminal with the Circleback CLI. (`/releases/circleback-cli`)
- **March 11, 2026** — Dark mode on mobile — Dark mode is now available on mobile. (`/releases/dark-mode-on-mobile`)
- **March 10, 2026** — Improved share modal — We've updated the share modal to make it easier to share meetings with the right people. (`/releases/improved-share-modal`)
- **March 9, 2026** — Circleback in ChatGPT — You can now access context from your conversations in Circleback directly in ChatGPT. (`/releases/circleback-in-chatgpt`)
- **March 9, 2026** — Change microphone and screen while recording — You can now switch your microphone and captured window while a recording is in progress on desktop. (`/releases/change-microphone-and-screen-while-recording`)
- **March 5, 2026** — Choose whether to record meeting video on desktop — You can now choose whether video from the meeting is included by default when recording with the desktop app. (`/releases/choose-whether-to-record-meeting-video-on-desktop`)
- **March 3, 2026** — Automatically end recordings on microphone use — The desktop app now automatically ends recordings when your microphone is no longer in use (e.g. (`/releases/automatically-end-recordings-on-microphone-use`)
- **March 2, 2026** — Change calendar event while recording — Hit record and notice your meeting is tied to the wrong calendar event? (`/releases/change-calendar-event-while-recording`)
- **February 26, 2026** — Quick navigation — The search modal now doubles as a way to navigate anywhere in Circleback. (`/releases/quick-navigation`)
- **February 20, 2026** — Collapsible sidebar — You can now collapse the navigation sidebar for a more focused view anywhere in Circleback. (`/releases/collapsible-sidebar`)
- **February 18, 2026** — Accessibility improvements — We've made improvements across the app to improve the accessibility of buttons and other input controls. (`/releases/accessibility-improvements`)
- **February 16, 2026** — Workspace admins can run automations for shared meetings — Workspace admins can now run automations on any meeting they have access to, not just meetings they own. (`/releases/workspace-admins-can-run-automations-for-shared-meetings`)
- **February 12, 2026** — Dark mode — Circleback now supports dark mode on web and desktop. (`/releases/dark-mode`)
- **February 11, 2026** — Hide solo meetings from the menu bar — You can now choose whether meetings with no invitees show up in your menu bar. (`/releases/hide-solo-meetings-from-the-menu-bar`)
- **January 30, 2026** — Recording retention increased to 2 years — Your meeting recordings are now kept for two years, up from one. (`/releases/recording-retention-increased-to-2-years`)
- **January 29, 2026** — Manage tags — You can now rename tags, change their color, add a description for automatic meeting tagging, and delete tags you no longer need from Settings → Tags. (`/releases/manage-tags`)
- **January 27, 2026** — New design — We've refreshed the look and feel of Circleback. (`/releases/new-design`)
- **January 13, 2026** — Circleback MCP — You can now connect AI tools to Circleback using MCP. (`/releases/circleback-mcp`)
- **January 7, 2026** — Faster filtering with your keyboard — You can now quickly filter meetings and action items without leaving your keyboard. (`/releases/faster-filtering-with-your-keyboard`)
- **December 23, 2025** — Claim ownership of meetings from inactive workspace members — When viewing sharing options for a shared meeting created by an inactive member of your workspace, you'll now see an option to claim ownership of that meeting. (`/releases/claim-ownership-of-meetings-from-inactive-workspace-members`)
- **December 11, 2025** — Start recording with widgets and the Action button on mobile — We've made it even faster to start recording meetings on mobile. (`/releases/start-recording-with-widgets-and-the-action-button-on-mobile`)
- **December 8, 2025** — Circling back on 2025 — It's time to circle back on 2025… Open Circleback to see your year in meetings. (`/releases/circling-back-on-2025`)
- **December 5, 2025** — Filter meetings by company — You can now filter meetings by company from the meetings page. (`/releases/filter-meetings-by-company`)
- **November 30, 2025** — Usage dashboard — You can now see how you and your team spend time in meetings with the new usage dashboard. (`/releases/usage-dashboard`)
- **November 18, 2025** — Ask Circleback in Slack — You can now ask Circleback questions without leaving Slack. (`/releases/ask-circleback-in-slack`)
- **November 10, 2025** — Rerun automations — You can now rerun an automation for any meeting–useful when you've updated an automation or meeting. (`/releases/rerun-automations`)
- **October 31, 2025** — Organize your workspace into teams — We've evolved our structure: what was previously called 'teams' are now 'workspaces,' and you can now organize your workspace into multiple teams with separate access and automations. (`/releases/organize-your-workspace-into-teams`)
- **October 28, 2025** — Edit speaker names for individual transcript segments — You can now edit the speaker name for individual transcript segments. (`/releases/edit-speaker-names-for-individual-transcript-segments`)
- **October 28, 2025** — Connect your email — You can now connect your email to Circleback to pull in context from email conversations alongside your meetings. (`/releases/connect-your-email`)
- **October 22, 2025** — Synced transcripts on mobile — You can now follow along as meeting recordings play on mobile. (`/releases/synced-transcripts-on-mobile`)
- **October 14, 2025** — Update deals in Attio — You can now update deals in Attio with meeting notes, action items, and insights using automations. (`/releases/update-deals-in-attio`)
- **October 14, 2025** — Ask Circleback about your calendar — The Circleback Assistant can now answer questions about your calendar. (`/releases/ask-circleback-about-your-calendar`)
- **September 30, 2025** — Add your own notes on mobile — You can now add your own notes to meetings while on the go. (`/releases/add-your-own-notes-on-mobile`)
- **September 25, 2025** — Share and run automations for multiple meetings — You can now quickly share, remove access, and run automations for multiple meetings at once. (`/releases/share-and-run-automations-for-multiple-meetings`)
- **September 23, 2025** — Weekly roundup — Stay on top of everything happening across conversations with the new weekly roundup. (`/releases/weekly-roundup`)
- **September 16, 2025** — Recording notifications on desktop — We've added a notification on desktop that lets you quickly set your recording preferences and record with one click when you join a meeting. (`/releases/recording-notifications-on-desktop`)
- **September 9, 2025** — Raycast extension — You can record and search meetings from Raycast with the new Circleback extension. (`/releases/raycast-extension`)
- **August 24, 2025** — Select and take actions on multiple meetings — You can now select and take actions on multiple meetings from the meetings page. (`/releases/select-and-take-actions-on-multiple-meetings`)
- **August 21, 2025** — Ask Circleback Assistant product questions — You can now ask Circleback Assistant about using Circleback and get immediate answers without needing to search through support articles. (`/releases/ask-circleback-assistant-product-questions`)
- **August 14, 2025** — Add your own notes — You can now add your own context to any meeting–before it starts or while it's happening. (`/releases/add-your-own-notes`)
- **August 13, 2025** — Share meetings on mobile — You can now share your meetings directly from the mobile app. (`/releases/share-meetings-on-mobile`)
- **August 13, 2025** — Circleback Assistant on mobile — Starting today, you can use Circleback Assistant on mobile to instantly find answers across all your meetings while on the go. (`/releases/circleback-assistant-on-mobile`)
- **August 6, 2025** — Automatic participant identification for hybrid meetings — When multiple people join a meeting from the same device–whether they're in a conference room or sharing a laptop–Circleback now automatically identifies and distinguishes between different speakers. (`/releases/automatic-participant-identification-for-hybrid-meetings`)
- **August 4, 2025** — Share meetings with invited team members automatically — You can now automatically share meetings only with team members who were invited to the calendar event. (`/releases/share-meetings-with-invited-team-members-automatically`)
- **July 29, 2025** — Choose joining preferences for recurring meetings — You can now choose whether Circleback automatically joins all occurrences of a recurring meeting. (`/releases/choose-joining-preferences-for-recurring-meetings`)
- **July 28, 2025** — Ask to join team — We've made it easier for colleagues to join your team. (`/releases/ask-to-join-team`)
- **July 21, 2025** — Automatically start recordings on microphone use — The desktop app can now automatically start recordings when your computer's microphone is accessed by an app. (`/releases/automatically-start-recordings-on-microphone-use`)
- **July 17, 2025** — Circleback Assistant — Today we're releasing Circleback Assistant, a powerful new assistant experience that works across all meetings. (`/releases/circleback-assistant`)
- **July 2, 2025** — Filter action items by tag — You can now filter action items by tag. (`/releases/filter-action-items-by-tag`)
- **June 30, 2025** — Search meetings with people and companies — You can now add people and companies to your search. (`/releases/search-meetings-with-people-and-companies`)
- **June 17, 2025** — Improved meeting assistant email drafts — We've made it easier to write and send emails with the meeting assistant. (`/releases/improved-meeting-assistant-email-drafts`)
- **June 16, 2025** — Mark action items done from email — Recipients of meeting notes emails can now mark their assigned action items done directly from the email by selecting the checkbox next to each item. (`/releases/mark-action-items-done-from-email`)
- **June 6, 2025** — View calendar event descriptions and email invitees — You can now view calendar event descriptions and email invitees of upcoming meetings from Home. (`/releases/view-calendar-event-descriptions-and-email-invitees`)
- **June 5, 2025** — Record panel dark mode — The record panel on desktop now supports dark mode, automatically switching based on your system settings on macOS and Windows. (`/releases/record-panel-dark-mode`)
- **June 5, 2025** — Control recordings using the Live Activity on iOS — The Live Activity on iOS has been updated in the latest version of the Circleback mobile app. (`/releases/control-recordings-using-the-live-activity-on-ios`)
- **May 31, 2025** — Meeting clips — You can now create and share clips from your meetings. (`/releases/meeting-clips`)
- **May 30, 2025** — Search on mobile — Search is now available on mobile, allowing you to quickly find any meeting, moment, or answer using the context of your meetings while on the go. (`/releases/search-on-mobile`)
- **May 28, 2025** — Search meetings with tags — You can now type @ to add tags to search. (`/releases/search-meetings-with-tags`)
- **May 28, 2025** — Edit meetings on mobile — You can now edit meeting names, notes, and action items on mobile. (`/releases/edit-meetings-on-mobile`)
- **May 21, 2025** — Salesforce integration now creates events — We've updated the Salesforce integration to create an event instead of a note for matching contacts and opportunities in Salesforce. (`/releases/salesforce-integration-now-creates-events`)
- **April 30, 2025** — Duplicate an automation — You can now duplicate an automation. (`/releases/duplicate-an-automation`)
- **April 30, 2025** — Create tasks in HubSpot — You can now create tasks in HubSpot with automations. (`/releases/create-tasks-in-hubspot`)
- **April 23, 2025** — Updated meeting notes editor — The meeting notes editor has been updated to make it easier to navigate and edit your notes. (`/releases/updated-meeting-notes-editor`)
- **April 23, 2025** — Automatically add new users to your team — New users who sign up for Circleback with an email matching your team’s domain can now be added to your team automatically. (`/releases/automatically-add-new-users-to-your-team`)
- **April 22, 2025** — View meetings relevant to an automation — You can now view all meetings that match an automation’s conditions. (`/releases/view-meetings-relevant-to-an-automation`)
- **April 17, 2025** — Mark all your action items done — You can now mark all your action items done in one go. (`/releases/mark-all-your-action-items-done`)
- **April 14, 2025** — Choose which action items get created in Linear and monday.com — You can now choose which action items are created in Linear and monday.com using automations. (`/releases/choose-which-action-items-get-created-in-linear-and-monday-com`)
- **April 9, 2025** — Improved Notion database integration — When integrating with Notion databases using automations, setting the page content to include action items or insights that result in multiple outcomes per meeting will no longer create one database row per action item/insight result. (`/releases/improved-notion-database-integration`)
- **April 8, 2025** — Saved meeting assistant chat history — The meeting assistant now saves your chat history, allowing you to return to any meeting and pick up where you left off with the assistant instead of starting from scratch. (`/releases/saved-meeting-assistant-chat-history`)
- **April 7, 2025** — Android app — Circleback is now available for Android. (`/releases/android-app`)
- **April 4, 2025** — Enable automations for your team — Admins can now enable automations for the entire team or specific members, making it easier to set up team-wide automations. (`/releases/enable-automations-for-your-team`)
- **April 3, 2025** — Meeting invitee automation conditions — We've added new automation trigger conditions to enable running automations based on the email addresses and email domains of the people invited to a meeting. (`/releases/meeting-invitee-automation-conditions`)
- **March 26, 2025** — Team default settings — Admins can now configure default and enforced settings for the team. (`/releases/team-default-settings`)
- **March 25, 2025** — Sources for meeting assistant answers — The meeting assistant now shows the specific parts of the meeting used as a source for an answer. (`/releases/sources-for-meeting-assistant-answers`)
- **March 24, 2025** — Recording Live Activity on iOS — The latest version of the Circleback mobile app for iOS (1.5.0) displays a Live Activity when using the app to record. (`/releases/recording-live-activity-on-ios`)
- **March 20, 2025** — Log in with Google and Apple — We've added Google and Apple login options to make accessing your Circleback account even easier. (`/releases/log-in-with-google-and-apple`)
- **March 13, 2025** — Automatically start and end recordings on desktop — We've introduced new settings in the desktop app that allow you to automatically start and end meeting recordings based on events on your calendar. (`/releases/automatically-start-and-end-recordings-on-desktop`)
- **March 6, 2025** — Record panel — We've released the record panel to make it easier to capture meetings with the desktop app. (`/releases/record-panel`)
- **March 5, 2025** — Associate linked meetings with calendar events — You can now associate meetings you add Circleback to via a link with calendar events. (`/releases/associate-linked-meetings-with-calendar-events`)
- **February 27, 2025** — Play recordings in the background on mobile — Meeting recordings on the mobile app will now continue playing after you leave the app, allowing you to listen in the background. (`/releases/play-recordings-in-the-background-on-mobile`)
- **February 27, 2025** — Offline recording on mobile — Recordings can now be started and finished on the mobile app while offline. (`/releases/offline-recording-on-mobile`)
- **February 25, 2025** — HubSpot integration now logs meetings — We've updated the HubSpot integration to log a meeting, instead of creating a note, for matching contacts and deals in HubSpot. (`/releases/hubspot-integration-now-logs-meetings`)
- **February 21, 2025** — Recording improvements — Recordings on web, desktop, and mobile now start immediately after the button is pressed, no more waiting around for the spinner. (`/releases/recording-improvements`)
- **February 19, 2025** — iOS app — Circleback is now available for iOS. (`/releases/ios-app`)
- **February 6, 2025** — Associate imported meetings with calendar events — You can now associate imported meetings with calendar events. (`/releases/associate-imported-meetings-with-calendar-events`)
- **January 28, 2025** — Real-time meeting assistant responses — We've enhanced the meeting assistant to provide faster responses. (`/releases/real-time-meeting-assistant-responses`)
- **January 24, 2025** — Improved automation trigger conditions — We've improved automations to give you more control over when they run. (`/releases/improved-automation-trigger-conditions`)
- **January 15, 2025** — Open meeting calendar event — You can now open a meeting's calendar event in Google Calendar or Microsoft Outlook by right-clicking or selecting ⋯ on the meeting. (`/releases/open-meeting-calendar-event`)
- **January 13, 2025** — Improvements to the meetings page — We've updated the meetings page to make it easier to navigate your meetings. (`/releases/improvements-to-the-meetings-page`)
- **January 6, 2025** — Zapier integration — Circleback now integrates directly with Zapier to connect with thousands of apps. (`/releases/zapier-integration`)
- **January 3, 2025** — Insight templates — We've introduced insight templates to help you get started faster. (`/releases/insight-templates`)
- **December 31, 2024** — Open meeting links in desktop app — You can now open Circleback meeting links in the desktop app. (`/releases/open-meeting-links-in-desktop-app`)
- **December 12, 2024** — Webhook integration in Automations — Automations now support webhook integration. (`/releases/webhook-integration-in-automations`)
- **December 9, 2024** — Create a new tag from the meeting context menu — You can now create a new tag from the meeting context menu. (`/releases/create-a-new-tag-from-the-meeting-context-menu`)
- **December 5, 2024** — Include recording and transcript access in invitee emails — You can now choose to include a link to the meeting recording (if available) and transcript in emails sent to invitees. (`/releases/include-recording-and-transcript-access-in-invitee-emails`)
- **December 3, 2024** — Circling back on 2024 — If you've been using Circleback this year, we have a special end-of-year surprise for you. (`/releases/circling-back-on-2024`)
- **November 20, 2024** — Automatic participant voice identification — Starting today, when you update a participant's name in a recorded meeting, their voice will be saved and automatically identified in future meetings they're invited to. (`/releases/automatic-participant-voice-identification`)
- **November 18, 2024** — Custom meeting participant image — You can now set a custom image for the Circleback bot participant in your meetings. (`/releases/custom-meeting-participant-image`)
- **November 14, 2024** — View all search results — You can now see all results for search across meeting names, invitees, and specific moments or mentions of phrases in your meetings. (`/releases/view-all-search-results`)
- **November 13, 2024** — Email receipts — You can now receive the invoice and receipt for each payment in your inbox by enabling email receipts from Settings → Billing. (`/releases/email-receipts`)
- **November 1, 2024** — Workflows are now Automations — We've renamed Workflows to Automations to more clearly communicate what they do: automatically find and take action on specific details from meetings that matter most to you and your teams. (`/releases/workflows-are-now-automations`)
- **October 30, 2024** — Tag upcoming meetings — You can now tag upcoming meetings by selecting ⋯ or right-clicking a meeting and selecting a tag. (`/releases/tag-upcoming-meetings`)
- **October 25, 2024** — Assistant improvements: resize and copy — The meeting assistant is now resizable, simply drag the left border to adjust its width. (`/releases/assistant-improvements-resize-and-copy`)
- **October 25, 2024** — Add viewers to your team — We've made it easier to share meetings with everyone on your team. (`/releases/add-viewers-to-your-team`)
- **October 10, 2024** — Update account email — You can now update the email address associated with your Circleback account from Settings → Account. (`/releases/update-account-email`)
- **October 7, 2024** — Improved transcript search — We've added keyword search to meeting transcripts. (`/releases/improved-transcript-search`)
- **September 30, 2024** — Share meetings in workflows — You can now use workflows to automatically share meetings with your team. (`/releases/share-meetings-in-workflows`)
- **September 26, 2024** — Data retention settings — You can now configure how long your team's data is stored on Circleback with the new data retention settings. (`/releases/data-retention-settings`)
- **September 17, 2024** — Support for new English, French, and Portuguese language locales — We've added support for the following new language locales in: English (United Kingdom) English (United States) French (France) French (Canada) Portuguese (Portugal) Portuguese (Brazil) To update the language used for meeting outcomes (i.e. (`/releases/support-for-new-english-french-and-portuguese-language-locales`)
- **September 16, 2024** — HubSpot deals and Salesforce opportunities integration improvements — Using the HubSpot and Salesforce integrations, you can now push meeting notes, action items, and insights to a HubSpot deal or Salesforce opportunity that is associated with a contact, company, or account. (`/releases/hubspot-deals-and-salesforce-opportunities-integration-improvements`)
- **September 16, 2024** — Desktop app — Today, we're releasing the Circleback desktop app for macOS and Windows. (`/releases/desktop-app`)
- **September 9, 2024** — Search for meetings by invitee email and domain — You can now search for meetings with specific invitee emails or domains. (`/releases/search-for-meetings-by-invitee-email-and-domain`)
- **September 5, 2024** — Create tasks in Attio — Starting today, you can use the Attio integration in Circleback to automatically create tasks linked to people or companies in Attio. (`/releases/create-tasks-in-attio`)
- **August 22, 2024** — Select microphone used for recordings — When recording a meeting on Circleback, you can now open the microphone dropdown to select which microphone is used for the recording. (`/releases/select-microphone-used-for-recordings`)
- **August 15, 2024** — Associate recorded meetings with calendar events — You can now associate meetings that you record on Circleback with events on your calendar. (`/releases/associate-recorded-meetings-with-calendar-events`)
- **August 7, 2024** — Increased import limit — We've increased the limit for importing meetings from 500 MB/130 minutes to a simple limit of 2 GB. (`/releases/increased-import-limit`)
- **August 6, 2024** — monday.com integration — Circleback is now available on the monday.com marketplace and integrates with monday.com to automatically create items for each action item from your meetings. (`/releases/monday-com-integration`)
- **August 2, 2024** — Recent meetings in details panel — Starting today, you'll see recent meetings with the same invitees in the meeting details panel. (`/releases/recent-meetings-in-details-panel`)
- **August 2, 2024** — Better search — We've made improvements to searching your meetings. (`/releases/better-search`)
- **August 1, 2024** — Workflow context menu — Building on the recent release for meetings, you can now right-click a workflow on the workflows page to quickly share or delete it. (`/releases/workflow-context-menu`)
- **August 1, 2024** — Streamed recordings — We've made recording meetings with Circleback more robust. (`/releases/streamed-recordings`)
- **July 30, 2024** — Meeting context menu — You can now right-click a meeting on the meetings page to quickly share it, run a workflow, or delete it. (`/releases/meeting-context-menu`)
- **July 28, 2024** — Share workflows with your team — Starting today, you can share workflows to simplify automations across your team. (`/releases/share-workflows-with-your-team`)
- **July 17, 2024** — Meeting details panel — You can now view information about your past and upcoming meetings in the new meeting details panel. (`/releases/meeting-details-panel`)
- **July 12, 2024** — Add action items to meetings — You can now add action items to your meetings in Circleback in addition to the ones that were automatically identified in the meeting. (`/releases/add-action-items-to-meetings`)
- **July 10, 2024** — Secondary calendars — Starting today, you can connect a secondary Google or Microsoft Outlook calendar to your Circleback account. (`/releases/secondary-calendars`)
- **June 21, 2024** — Unified meeting records for teams — We've made it simpler for teams on Circleback to manage their meetings. (`/releases/unified-meeting-records-for-teams`)
- **June 19, 2024** — Persisting filters — Filters applied to meetings or action items now persist while you're signed in. (`/releases/persisting-filters`)
- **June 18, 2024** — Search for team members when sharing a meeting — When sharing a meeting, you can now search to quickly find the team members you want to share the meeting with. (`/releases/search-for-team-members-when-sharing-a-meeting`)
- **June 18, 2024** — Meetings tab update: your own and shared meetings in one place — All of your meetings are now accessible in the Meetings tab. (`/releases/meetings-tab-update-your-own-and-shared-meetings-in-one-place`)
- **June 16, 2024** — One Circleback bot participant per meeting — Starting today, for meetings with multiple invitees using Circleback, only one Circleback bot participant will join the call, instead of one bot per user. (`/releases/one-circleback-participant-per-meeting`)
- **June 12, 2024** — Support for Zoom webinars — You can now use Circleback in Zoom meetings and webinars that require registration. (`/releases/support-for-zoom-webinars`)
- **June 12, 2024** — Improved insights table — We've improved how insights are displayed and can be accessed on Circleback. (`/releases/improved-insights-table`)
- **June 11, 2024** — Participant speaking durations — You can now see how much time each participant spent speaking during a meeting by opening the participants panel. (`/releases/participant-speaking-durations`)
- **May 29, 2024** — Transcript tab updates — We've released several improvements to the transcript tab to make your meeting playback experience better. (`/releases/transcript-tab-updates`)
- **May 29, 2024** — Send to Slack in threads — If you've set up a workflow in Circleback to send notes to a Slack channel, they'll now show up there formatted as a thread. (`/releases/send-to-slack-in-threads`)
- **May 24, 2024** — Run workflows manually — You can now manually run workflows for a meeting. (`/releases/run-workflows-manually`)
- **May 23, 2024** — Support for adding notes to companies on Attio — When integrating with Attio in a workflow, you can now choose to have notes automatically added to companies after meetings. (`/releases/support-for-adding-notes-to-companies-on-attio`)
- **May 17, 2024** — Referral link and track your free months — Telling someone about Circleback? (`/releases/referral-link-and-track-your-free-months`)
- **May 17, 2024** — Notion database integration — You can now use workflows in Circleback to automatically update Notion databases and tables. (`/releases/notion-database-integration`)
- **May 3, 2024** — Improved meeting sharing — We've added more control over how you can share your meetings, including the ability to manually email meeting notes to people you met with. (`/releases/improved-meeting-sharing`)
- **May 1, 2024** — Automatic joining preference for meetings hosted by you — You can now choose to have Circleback only join meetings you create on your calendar. (`/releases/automatic-joining-preference-for-meetings-hosted-by-you`)
- **April 24, 2024** — Get free Circleback with referrals — Starting today, you earn a free month for each person who joins through the meeting notes emails sent to attendees. (`/releases/get-free-circleback-with-referrals`)
- **April 17, 2024** — Webhook signature verification — To ensure the the webhook events received are from Circleback, you can now choose to verify incoming requests using a signing secret. (`/releases/webhook-signature-verification`)
- **April 17, 2024** — HubSpot and Salesforce integration improvements — You can now choose to add notes directly to deals in HubSpot or opportunities in Salesforce. (`/releases/hubspot-and-salesforce-integration-improvements`)
- **April 10, 2024** — Improved mobile web experience — The new mobile web experience allows you to access all the features of Circleback directly from your phone. (`/releases/improved-mobile-web-experience`)
- **April 9, 2024** — Team admin and billing owner controls — You can now grant admin privileges or transfer billing ownership to other team members from Settings → Team. (`/releases/team-admin-and-billing-owner-controls`)
- **April 5, 2024** — Number of insights per meeting — You can now choose the number of instances of an insight you want identified per meeting. (`/releases/number-of-insights-per-meeting`)
- **April 1, 2024** — Language setting — You can now choose the language your notes will be in. (`/releases/language-setting`)
- **April 1, 2024** — Copy improvements — You can now easily comment on or copy a highlighted piece of your transcript. (`/releases/copy-improvements`)
- **March 28, 2024** — Zoho CRM integration — Circleback now integrates with Zoho to automatically update your CRM with notes, action items, and other insights from your meetings. (`/releases/zoho-crm-integration`)
- **March 26, 2024** — Attio integration — Circleback now integrates with Attio to automatically update your CRM with notes, action items, and other insights from your meetings. (`/releases/attio-integration`)
- **March 13, 2024** — Comments on transcript — You can now highlight moments and leave comments for your team for each of your meetings. (`/releases/comments-on-transcript`)
- **March 12, 2024** — Custom fields for insights — You can now customize the exact structure for the insights you set up in workflows. (`/releases/custom-fields-for-insights`)
- **February 26, 2024** — Shareable moments — You can now share a specific moment in your meetings by copying a link to it. (`/releases/shareable-moments`)
- **February 25, 2024** — Insights tab — We've added an Insights tab, where you can see all of the insights from across your meetings in one place. (`/releases/insights-tab`)
- **February 16, 2024** — Download meeting recording and increased retention — We've introduced the ability to download video or audio of meetings with saved recordings by clicking the button in the top-right corner of the player in the meeting transcript tab. (`/releases/download-meeting-recording-and-increased-retention`)
- **February 15, 2024** — Workflows — Your meetings have a lot of useful information in them (a goldmine, if you will). (`/releases/workflows`)
- **February 14, 2024** — Slack huddles support — Circleback now supports Slack huddles! (`/releases/slack-huddles-support`)
- **February 9, 2024** — Review last meeting — Ever joined a meeting without a clue about where things were left last time? (`/releases/review-last-meeting`)
- **January 23, 2024** — Automatically tag meetings — You can now have your meetings automatically tagged. (`/releases/automatically-tag-meetings`)
- **January 5, 2024** — Mobile-optimized meeting page — With the introduction of shareable meeting links, we wanted to make sure that the links you share look good regardless of where they're opened. (`/releases/mobile-optimized-meeting-page`)
- **January 4, 2024** — Shareable meeting links — Starting today, users on a team plan can create and share a link that provides those outside the team access to view the meeting on Circleback. (`/releases/shareable-meeting-links`)
- **December 24, 2023** — Improved meeting assistant and search — We've made changes that improve the quality of responses when using the meeting assistant and search. (`/releases/improved-meeting-assistant-and-search`)
- **December 19, 2023** — Custom vocabulary — Starting today, you can define your custom vocabulary to improve the accuracy of transcribing specific words, names, phrases, or technical terms. (`/releases/custom-vocabulary`)
- **December 15, 2023** — Participant colours in the transcript — We've made it easier to read through and navigate a transcript by using a unique colour for each participant. (`/releases/participant-colours-in-the-transcript`)
- **December 14, 2023** — Save and replay meeting recordings — Starting today, you can save and replay audio and video recordings from your meetings. (`/releases/save-and-replay-meeting-recordings`)
- **December 5, 2023** — Copy notes and transcript — No more drag clicking down multiple pages to copy notes. (`/releases/copy-notes-and-transcript`)
- **November 30, 2023** — Invite team members to Circleback — Beginning today, you can add members from your team to Circleback without them needing to have a Circleback account first. (`/releases/invite-team-members-to-circleback`)
- **November 30, 2023** — Action items tab — Starting today, you can view and manage action items for yourself, your team, and people you've met with from one place. (`/releases/action-items-tab`)
- **November 21, 2023** — Faster login emails — We've made improvements that result in login emails arriving in your inbox much faster than before. (`/releases/faster-login-emails`)
- **November 14, 2023** — Record conversations — Following the release of our import feature, today's we're releasing the ability to record conversations directly from Circleback on desktop and mobile. (`/releases/record-conversations`)
- **October 23, 2023** — Import meetings by uploading audio and video files — We've heard you loud and clear. (`/releases/import-meetings-by-uploading-audio-and-video-files`)
- **September 19, 2023** — Faster navigation — We've shipped performance improvements that make navigating Circleback much snappier. (`/releases/faster-navigation`)
- **August 29, 2023** — New toggles for calendar and sharing — We're dubbing today Toggle Tuesday because we're releasing two new toggles. (`/releases/new-toggles-for-calendar-and-sharing`)
- **August 15, 2023** — Webhook integration — Our webhook integration is here! (`/releases/webhook-integration`)
- **July 24, 2023** — Action items — We're making it easier to manage action items from meetings, starting by automatically creating assigned action items. (`/releases/action-items`)
- **June 12, 2023** — Tagging, email notifications, and design improvements — We shipped a few things this week. (`/releases/tagging-email-notifications-and-design-improvements`)
- **June 2, 2023** — Share meetings with individual team members — Have a meeting that you don't want to share with the whole team? (`/releases/share-meetings-with-individual-team-members`)
- **May 8, 2023** — Upcoming meetings and filtering — Following up on the ability to connect your calendar, this week we released the ability to see the upcoming meetings on your calendar when you log on to Circleback. (`/releases/upcoming-meetings-and-filtering`)
- **April 27, 2023** — Connect your calendar — Starting today, you can connect your Google Calendar and Microsoft Outlook Calendar and have Circleback automatically join the meetings you choose. (`/releases/connect-your-calendar`)
- **April 3, 2023** — Meeting assistant — You can now hop into a conversation with your meeting. (`/releases/meeting-assistant`)
