# What users say about Granola, Circleback and Meetily (and open-source alternatives)

Research date: **2026-10-08**. Transcription accuracy and multilingual support are left out on purpose.

## 0. Method and source caveats (read first)

- **I could not read Reddit, G2 or Capterra directly.** Reddit refuses the research tools (both WebSearch `allowed_domains: reddit.com` and WebFetch failed), and G2 and Capterra returned HTTP 403. Reddit sentiment below is **second-hand**: it comes from blogs that quote or summarise threads, and most of those blogs are written by competitors. I mark these items **[secondary]**. To check them, search r/macapps, r/selfhosted, r/ProductivityApps and r/LocalLLaMA by hand.
- **The primary sources are:** the Meetily GitHub repo (issues, PRs, discussions and releases, pulled with `gh` on 2026-10-08), Hacker News threads (read through the Algolia API), Product Hunt reviews, and the official pricing, changelog and release pages of Granola, Circleback and Meetily.
- **Most "Granola vs X" and "best alternative" blogs are written by competitors.** That includes circleback.ai, anarlog.so, openwhispr.com, clearminutes.app, hirekai.ai, happyscribe.com, tldv.io, meetgeek.ai and meetily.ai. They are still useful for spotting complaint themes, but treat their claims as marketing.

---

## 1. Quick facts as of Oct 2026

| | **Granola** | **Circleback** | **Meetily** (Zackriya Solutions) |
|---|---|---|---|
| Capture | Bot-free capture of device audio. Desktop (Mac and Windows), iOS, Android (launched 2026-07-01) and Apple Watch (launched 2026-07-28) | Either a bot or the bot-free desktop app. Mobile and Apple Watch apps for in-person meetings. Web app | Bot-free capture of mic and system audio. macOS (Apple Silicon only) and Windows. Linux is build-from-source only |
| Pricing | Basic $0, which keeps only **30 days of history**. Business $14/user/mo. Enterprise $35 ([pricing](https://www.granola.ai/pricing)) | Free (30-day history, limited automations). Pro $15. Business $25. Enterprise $35 ([pricing](https://circleback.ai/pricing)) | Community edition is free under MIT. Pro is $10/user/mo **billed annually only** ($120/yr; "60% off" a stated $25). Enterprise is custom ([pro](https://meetily.ai/pro)) |
| Audio retained? | **No.** Audio is deleted after transcription | Yes. Recordings are optional, with playback and clips (1 yr on Pro, 2 yrs on Business) | Yes, stored locally |
| AI chat over meetings | Yes, within one meeting and across meetings. Chat was rebuilt as an agent with citations on 2026-04-21 | "Ask Circleback" does agentic search with citations, on all plans | **No.** The Pro pricing page lists it as "Soon". Community PRs #770 and #810 are still open |
| MCP/API | MCP shipped 2026-02-04. API on Business plan | API, MCP and CLI on all plans (read-only) | Pro 1.11.0 (2026-09-24) added MCP, signed webhooks and a CLI |
| Speaker labels | Desktop only separates "Me" from "Them". iPhone labels up to 10 speakers | On every capture route | Pro only, since 1.8.2 (2026-07-22). Community PR #833 opened 2026-10-08 |
| GitHub | n/a | n/a | 31.5k stars, 3.4k forks, 200+ open issues, MIT |

---

## 2. Granola

### 2.1 Killer features (why people pay)
1. **No bot in the call.** This is the most-cited reason people choose Granola. Product Hunt reviewers in 2025-26 wrote "No bot joins the call, which sounds cosmetic until you're on a founder call" (Asad M., about 1 month before 2026-10) and "I hate having a bot in my meetings" (Evan Taylor, PH discussion) ([PH reviews](https://www.producthunt.com/products/granola/reviews), [PH discussion](https://www.producthunt.com/p/granola/with-so-many-new-ai-note-takers-what-s-your-favorite-and-why)).
2. **A notepad you write in, which the AI then enhances.** Your own typed notes steer the summary and stay next to the transcript. Reviewers call the result "concise, well-structured notes that are easy to scan" ([arrows.to 22-tool sales test](https://arrows.to/guide/top-ai-notetakers/a-side-by-side-comparison-of-22-ai-notetakers-for-sales/)). Chris Messina uses Granola "for high-level summaries" and Fathom for specifics.
3. **Calm, minimal UI that sits on the desktop.** It is "less intrusive in meetings as it sits on your desktop" ([PH discussion](https://www.producthunt.com/p/granola/with-so-many-new-ai-note-takers-what-s-your-favorite-and-why)).
4. **Value builds up through chat across meetings, folders and people.** "The real benefit of Granola compounds over time" (Andrea Del Angel, PH, ~Apr 2026). The roadmap backs this up: cross-meeting chat (May 2025), Granola 2.0 shared team folders (May 2025), agentic Chat with **Recipes** and inline citations (2026-04-21), **Briefs** before meetings (2026-05-20), MCP for Claude, ChatGPT and Cursor (2026-02-04), follow-up email drafts, and a workspace dictionary ([updates](https://www.granola.ai/updates)).
5. **Templates and Recipes.** Users save prompts, such as follow-up emails or coaching, and reuse them across meetings.
6. **Coverage across devices.** Apps for Mac, Windows, iOS, Android and Apple Watch, plus iPhone phone-call notes (2026).

### 2.2 Top complaints
| Complaint | Evidence (date) |
|---|---|
| **Free tier hides history older than 30 days**, and CSV export leaves out transcripts | Pricing page. [hirekai.ai](https://hirekai.ai/blog/granola-review) (2026-09-03). [anarlog complaints](https://anarlog.so/blog/granola-ai-complaints/) (2026-08-30, competitor). On HN, the All-Ears author wrote that "Granola started clamping down on downloading your notes without paying" ([HN 49091894](https://news.ycombinator.com/item?id=49091894), 2026-07-29) |
| **Speaker attribution is weak.** Desktop only knows "Me/Them" | "Speaker attribution on a four person call is still a coin flip" (PH, ~Sep 2026). Circleback's comparison page and anarlog say the same (both competitors) |
| **No audio to check the transcript against.** Audio is deleted, and there are no clips or playback | tl;dv, hirekai and happyscribe reviews (2026). One PH discussion user switches to another tool "to dig in" because of this |
| **Silent capture failures.** The app looks like it is recording but captures nothing | [secondary] anarlog cites a founder who got notes for "about 60% of calls" ([anarlog](https://anarlog.so/blog/granola-ai-complaints/), 2026-08-30) |
| **Integrations are thin and paywalled.** Notion, HubSpot and Zapier need Business. Tasks only reach Asana, Linear or Jira through Zapier, with no trigger that fires on every meeting | [secondary] Reddit user asking how to sync to Google Docs: "Zapier or nothing" ([anarlog](https://anarlog.so/blog/granola-ai-complaints/)). [hirekai](https://hirekai.ai/blog/granola-review) (2026-09-03): "everything Granola produces stays inside the notes app". The HN Show post "How I turned Granola action items into to-do list" ([HN 49491815](https://news.ycombinator.com/item?id=49491815), 2026-08-29) is a user building the missing task layer |
| **Privacy defaults.** Note links were "viewable to anyone with the link" by default, and model training is opt-out on non-Enterprise plans | The Verge PSA, 2026-04-02 ([summary](https://www.wilsonsmedia.com/psa-anyone-with-a-link-can-view-your-granola-notes-by-default/), [techbuzz](https://www.techbuzz.ai/articles/granola-s-private-ai-notes-are-public-by-default)). An HN commenter on the Hyprnote launch was "extremely disturbed" that folders were public to the whole org, including 1:1s ([HN 44725306](https://news.ycombinator.com/item?id=44725306), 2025-07-30) |
| **Security incident.** A one-click account or session takeover was disclosed 2026-07-22 | [HN 48993371](https://news.ycombinator.com/item?id=48993371), [HN 49007641](https://news.ycombinator.com/item?id=49007641), [strix.ai writeup](https://www.strix.ai/blog/granola) |
| **Consent and legal exposure.** Other participants are not told they are being recorded. "Heads Up" notifications are an Enterprise pilot. A lawsuit, Chamberlain v. Granola, is cited | [secondary] Reddit "is it weird that it doesn't tell other people" ([anarlog](https://anarlog.so/blog/granola-ai-complaints/)) |
| **Trust in a free cloud app.** "Granola terrifies me... if something is free then my attention or my data tends to be the product" | [HN 49089026](https://news.ycombinator.com/item?id=49089026) (2026-07-28). The same thread asks for an Apple Watch app with "local (iPhone) or self-hosted processing" |
| **Pricing structure.** Solo tiers were retired, so users jump from Free straight to a $14 Business seat, and there is no annual discount | [hirekai](https://hirekai.ai/blog/granola-review), [itsconvo](https://www.itsconvo.com/blog/granola-ai-review) (2026) |
| **Too many tools and "summary whiplash"** | [secondary] r/ProductivityApps threads summarised by [remio](https://www.remio.ai/post/reddit-users-are-turning-granola-into-the-new-meeting-attention-debate) (Jun 2026): one sync created 4 Notion pages, 3 Linear issues and 2 Slack messages |

---

## 3. Circleback

### 3.1 Killer features (why people pay)
1. **Automations that carry the work past the meeting.** Conditional rules push notes and action items to Slack, HubSpot, Salesforce, Attio, Notion, Linear, Asana and webhooks. Rules can route by tag, attendee domain or name pattern, and can re-run on old meetings. Every review treats this as the main difference from Granola ([switchmytool](https://www.switchmytool.com/blog/circleback-ai-review), 2026-07-09: "worth testing if meeting follow-up is where your work leaks"). On the Launch HN, one customer said Circleback shipped HubSpot and Slack integrations soon after the customer had built them in-house ([HN 39618640](https://news.ycombinator.com/item?id=39618640)).
2. **Action items with owners and real dates.** A workspace-wide task list, plus next steps calculated to "specific dates instead of vague phrases" ([arrows.to](https://arrows.to/guide/top-ai-notetakers/a-side-by-side-comparison-of-22-ai-notetakers-for-sales/)).
3. **"Ask Circleback."** Agentic search across all meetings with cited answers, plus people and company pages that combine meetings and emails.
4. **Bot and bot-free capture both available.** The desktop app records without a bot, the bot is there for meetings you don't attend, and mobile and Apple Watch cover in-person meetings. One reviewer said it "automatically joins Teams or Zoom sessions, even if I'm not the organizer" [secondary].
5. **Stored recordings, playback and clips.** This is exactly what Granola lacks.
6. **Note and action-item quality.** [secondary] One Reddit user "rarely needs to verify" the notes ([switchmytool](https://www.switchmytool.com/blog/circleback-ai-review)).
7. **Team admin.** Auto-sharing rules, enforced default settings, retention controls, SOC 2 Type II and HIPAA BAAs.

### 3.2 Top complaints
| Complaint | Evidence |
|---|---|
| **Customer support, especially billing and cancellation** | A G2 review titled "Best AI notetaker with worst customer service" (0.5/5) [secondary via search]. Capterra: cancellations took weeks while charges continued ([happyscribe](https://www.happyscribe.com/blog/circleback-alternatives-for-ai-meeting-notes), 2026-03-06) |
| **Price for light users.** It used to have no free plan, only a trial. The current pricing page shows a Free tier | [switchmytool](https://www.switchmytool.com/blog/circleback-ai-review) (2026-07). Sources conflict on when the free plan launched (MeetGeek says Aug 2026) |
| **Bot friction and trust in the access model.** It asks for calendar, meeting and cloud permissions | [secondary] Reddit summary: "The doubt is about price, permissions, and whether you have enough meetings" |
| **Weak onboarding, and recurring meetings need approval each time** | Trustpilot via [happyscribe](https://www.happyscribe.com/blog/circleback-alternatives-for-ai-meeting-notes) (2026-03) |
| **Bugs in the iPhone app.** Speaker ID suffers in crowded rooms | G2 via search summary. [meetgeek](https://meetgeek.ai/blog/circleback-alternatives) (2026) |
| **Cost of switching from Granola.** Templates, vocabulary, sharing rules and automations all have to be rebuilt | [switchmytool alternatives](https://www.switchmytool.com/blog/best-granola-ai-alternatives) |

### 3.3 Granola vs Circleback in one line
Granola is the **calm notepad**: you write, the AI enhances, there is no bot, and the outputs stay inside the app. Circleback is the **follow-through engine**: recordings, assigned tasks, CRM and Slack automations, and auto-sharing. Users who love Granola still complain about integrations, audio and speaker labels, which are Circleback's strengths. Users who love Circleback complain about support and price ([CharterWorks/TIME](https://time.com/charter/6987522/the-best-ai-note-taking-tools-for-meetings/): Circleback is the best bot, Granola is for people who want to keep writing notes).

---

## 4. Meetily (Community Edition and PRO)

### 4.1 Release state
- **Community v0.4.1** (2026-09-12, [release](https://github.com/Zackriya-Solutions/meetily/releases/tag/v0.4.1)) is entirely a reliability release:
  - Bluetooth mic recovery on macOS
  - Short replies such as "yes" or "ok" are no longer dropped
  - Summaries of long meetings no longer lose transcript coverage, and Claude thinking blocks and Ollama reasoning are handled
  - Parakeet downloads can resume
  - VAD segmentation fixes
  - Resizable transcript and summary panes

  All 8 external contributors were first-timers. The release discussion (#799) asks users "What is the most frustrating reliability issue in your meeting workflow?"
- **v0.4.0** (2026-06-05):
  - Summaries in more languages
  - Analytics made **opt-in**
  - Close-to-tray
  - Single instance
  - Built-in Qwen 3.5 summary models
- **v0.3.0** (2026-03-03):
  - Audio import and retranscribe
  - Its "What's next" listed inline transcript editing, a post-processing (record-only) mode and dark mode. As of Oct 2026 these have shipped in Pro but **not in Community**; the matching issues #377, #338/#729/#677 and #355/#411 are still open.
- **Pro releases:**
  - 1.8.2 (2026-07-22) added **speaker diarisation**: live and on import, rename once to apply everywhere, merge duplicates, names remembered across meetings ([notes](https://meetily.ai/releases/pro/1.8.2)).
  - 1.11.0 (2026-09-24) added an **MCP server** that Claude Desktop, Claude Code, Codex, Cursor and Windsurf can use to search meetings and start or stop recording; **HMAC-signed webhooks** when a summary is ready; a `meetily-pro` **CLI** that exports Markdown, text or JSON; and `meetilypro://` deep links. It also stopped hallucinated attendees and dates ([notes](https://meetily.ai/releases/pro/1.11.0)).
  - Pro has had auto-detect meetings since about Dec 2025 (Windows since mid-Jan 2026), per a maintainer in #387.

### 4.2 PRO and Enterprise vs Community

[meetily.ai/pro](https://meetily.ai/pro), [pricing](https://meetily.ai/pricing), README, CEO blog of 2026-09-09 ([post](https://meetily.ai/blog/why-we-chose-experience-over-features)).

| Feature | Community (MIT, free) | PRO ($10/user/mo, annual) |
|---|---|---|
| Local live transcription (Whisper/Parakeet), import and retranscribe | ✅ | ✅ |
| AI summaries: Ollama, Claude, Groq, OpenRouter, OpenAI-compatible | ✅ | ✅ (+ "custom AI model connector") |
| Speaker diarisation and identification | ❌ (PR #833 is open) | ✅ since 1.8.2 |
| Custom summary templates | Default template only; custom JSON files work only as a workaround (#243) | ✅ 6 built-in + custom |
| Exports to PDF, DOCX, Markdown | ❌ (copy only; issue #816, PR #714) | ✅ |
| Auto-detect meetings with a prompt or countdown auto-record | ❌ (#218, #387; PR #733) | ✅ (fully hands-free auto-join still "in development" as of 2026-06-24) |
| Transcript editing, search and replace, audio player with seek | ❌ / partial | ✅ (since ~Mar 2026) |
| Windows NVIDIA GPU acceleration | Build from source only | ✅ |
| MCP, webhooks, CLI | ❌ (issue #595 asks for webhooks) | ✅ 1.11.0 |
| Chat with meetings | ❌ (PRs #770, #810) | "Soon" |
| Calendar integration | ❌ (#219, #449; PRs #666, #797) | "Soon" |
| Linux native build | Build from source | Waitlist |
| Support | Discord and GitHub | Priority email |

**Enterprise** ([page](https://meetily.ai/enterprise)) is for organisations of 100+ users and has three tiers: Standard, Premium and Ultimate. It offers:
- An admin dashboard with provisioning, RBAC, retention policies, an audit log viewer, usage analytics and health monitoring
- LDAP and Active Directory
- AES-256 encryption
- Air-gapped, on-prem or customer-cloud deployment, or EU-hosted options (Hetzner, OVH, LeaseWeb)
- Compliance positioning for GDPR, NIS2, HIPAA with a BAA, MiFID II and ISO 27001
- A named account manager and 24/7 support

### 4.3 Roadmap signals
- The README still lists Speaker ID, Chat with Meetings and Calendar Integration as "Coming Soon" for Pro. Speaker ID has in fact shipped. The README also still carries a stale "diarization mid-June" note and the LAUNCH20 coupon.
- On #218 (2026-05-18), a maintainer said: "Once we add few more compelling features like calendar integration and speaker diarization to the pro version, we are planning to release few pro features to OSS version as well... within 1 or 2 months." That has not happened as of Oct 2026.
- On #377 (2026-03-03), a maintainer listed upcoming Pro work: transcript edit, search and replace, meeting-detection UX, multi-language summaries, TXT export, and an audio player with seek. They said these would come to open source "in upcoming weeks". Multi-language summaries did reach Community in v0.4.0.
- The CEO post of 2026-09-09 sets the philosophy as "experience over features": make basics, onboarding and reliability solid before adding more. It names no upcoming features.
- Discussion #525 (Jun 2026) asked why paid features are gated in a local app. The maintainer defended open core ("Pro has maybe two or three prominent features") and **invited community PRs for auto-detect and auto-export**, provided they stay independent of the paid backend. The user replied that they would rather pay a **one-time or lifetime fee** ($50-60), because a SaaS subscription feels wrong for a local app. The maintainer said they are "exploring life time deals".

### 4.4 GitHub issues: what Community users ask for most

Open issues sorted by total reactions, then comments, pulled 2026-10-08 (200 open issues):

| # | Issue | Reactions | Comments | Notes |
|---|---|---|---|---|
| 226 / 230 | **Speaker identification / diarization** | 15 / 7 | 1 / **15** | The most-discussed feature. Users ask "Why is this feature only available in Pro?" (2026-03-05) and "this should be a very basic one" (2026-04-03). Cheap ideas raised: label the mic as "me" and system audio as "them"; one channel per mic. Community PR #833 opened today |
| 376 | Can't change the recordings folder in Community | 12 | 1 | Basic storage control |
| 243 | **Configurable summary templates / prompts** | 11 | 7 | Related: #235 (prompt the summary style), #422 (default template), discussion #276, discussion #313 (notes and context fed into the prompt) |
| 241 | Multitrack audio (separate mic and system tracks) | 9 | 0 | PR #823; #642 asks to attribute the transcript to mic vs system |
| 301 / 210 / 274 / 507 / 527 / 612 / 657 | **Remote or OpenAI-compatible transcription endpoints**, self-hosted ASR server, websocket streaming | 8 / 5 / 1… | 4 / 5… | Strong signal from r/selfhosted-type users: run the GPU box elsewhere. PR #837 opened today |
| 218 / 387 / 761 / 531 | **Auto-detect meeting, auto-record, auto-stop** | 6 / 2 | 5 / **11** | "This is the killer function for this. IF this was on, subscribe me up!" (#218, 2025-11-10). A consultant said "upgrading to the pro version is not an option" (#387). PR #733 |
| 474 / 824 | **Domain vocabulary and terminology corrections** | 6 | 2 | PR #824 adds a terminology-corrections tab |
| 377 / 357 / 753 | **Inline transcript editing, then re-summarize**; search and replace | 4 | 6 | A user said "I really like this over Granola, if you just make a 'click to edit'..." |
| 449 / 219 | **Calendar integration** (Google Calendar start and stop) | 4 / 3 | 1 / 2 | PRs #666 and #797 |
| 411 / 355 / 554 | Dark mode | 4 / 2 | 1 / 2 | |
| 250 / disc. #361 / #445 | Android and iPhone mobile app | 4 | – | |
| 338 / 729 / 677 | **Recording-only mode** (no live transcription, process afterwards) | 2 | **8** | Slow laptops. PR #532 |
| 424 | **Folders** to organise meetings | 3 | 0 | |
| 252 / disc. #313 | **Manual notes section** saved with the meeting, Granola-style | 2 | 1 | |
| 260 / PR #732 | Global hotkeys | 2 | 1 | |
| 548 / disc. #266 / PRs #770, #810 | **Chat with history (local RAG)** | 1 | 0 | A user who left Fireflies: "missing... ability to ask questions about a meeting" |
| 553 | Summary traceability, linking claims to evidence | 1 | 0 | |
| 595 / disc. #576 / #344 / #508 | **Outbound webhook, auto-export Markdown, Obsidian export** | – | – | Users have built their own Obsidian exporters (disc. #344, #508) |
| 432 | Cross-meeting memory via MCP | – | – | Now covered in Pro 1.11 |
| 816 / PR #714 | PDF and Word export | – | – | Pro-gated |
| 393 / 558 / 717 / 706 / 518 | Merge meetings, timeline navigation, meeting date in the summary, rename the folder on disk, delete files when deleting from the UI | – | – | Housekeeping and quality-of-life |

**Most-commented bugs** (themes): Local Whisper not using the GPU (#456, 11 comments; #333). Windows crashes and hangs at "Loading transcription engine" (#698, #228, #594, #700). Linux system audio, PipeWire, and builds that need source compilation (#383, #273, #701, #428, #305, #426, #697 asks for an AppImage or deb). Bluetooth mic on macOS (#450, fixed in 0.4.1). App fails to launch on macOS 26 (#726). Copy and paste broken in the summary editor (#752). Privacy policy says titles aren't collected but they are (#448). Pre-Haswell CPUs without AVX2 crash (#465). Intel Mac is unsupported (#77, #591).

**Open-core tension.** Many open community PRs re-implement Pro features: speakers (#833), calendar auto-record (#797, #666), auto-detect (#733), Ask-your-meetings (#770, #810), exports (#714), hotkeys (#732) and terminology (#824). Merging them, or not, is a visible community-relations decision.

### 4.5 What reviewers and users say about Meetily
- **Pros:**
  - Free under MIT, no account, bot-free, fully offline with Ollama
  - **Native Windows**, which "the single biggest practical advantage" calls rare among local-first tools (anarlog review, Apr 2026, quoted by [andrew.ooo](https://andrew.ooo/posts/meetily-privacy-first-ai-meeting-assistant-review/))
  - [secondary] r/selfhosted praise: "Finally, a meeting tool that doesn't require me to trust a VC-backed startup"
- **Cons:**
  - No calendar or auto-start, and basic or no diarization in Community
  - Summary quality drops on meetings over 90 minutes with small Ollama models
  - No sync or multi-device use
  - Linux is build-from-source only, and there is no Intel Mac build
  - No integrations (CRM, tasks), no guided onboarding, no vendor support for free users
  - Sources: [promptquorum](https://www.promptquorum.com/power-local-llm/meetily-review) (2026-09-18), [andrew.ooo](https://andrew.ooo/posts/meetily-privacy-first-ai-meeting-assistant-review/) (2026-07-06), [clearminutes](https://clearminutes.app/alternatives/meetily) (2026-05, competitor), [openwhispr](https://openwhispr.com/compare/meetily) (competitor)
- **Historic HN complaint:** "Meetily is nice but setup feels too convoluted, with a backend and frontend being required to separately install" ([HN 44725306](https://news.ycombinator.com/item?id=44725306), 2025-07-29). This was largely fixed by the single Tauri app in v0.1.x, but the reputation lingers.
- **Pricing pushback:** Annual-only billing, and the SaaS pricing feels wrong for a local app (disc. #525; openwhispr compare).

---

## 5. Other open-source and local Granola alternatives: features Meetily lacks

| Project | Status (Oct 2026) | Features worth borrowing that Meetily Community lacks |
|---|---|---|
| **Anarlog** (formerly Hyprnote, then Char) [repo](https://github.com/fastrepl/anarlog), 9.5k★, MIT, pushed 2026-10-08 | YC S25. Renamed 2026-05-03. The team's focus moved to the new "char" product, so Anarlog is community-maintained. On Ask HN (2026-04-09), a user said Char had "stopped supporting local models" ([HN 47701485](https://news.ycombinator.com/item?id=47701485)) | **A Granola-style notepad where your own notes steer the summary.** A Descript-like transcript editor for reassigning speakers. Native Apple Calendar integration. Export to Obsidian. Markdown files. Optional encrypted CloudSync and sharing. Local CLI and MCP server. iOS and Android betas. Watch companion. Consent options on the enterprise roadmap (chat message, silent bot, consent links). Pro sold as a $179/yr offline-verifiable license key with no sign-up (at launch) |
| **Muesli** [repo](https://github.com/Muesli-HQ/muesli), 1.4k★, macOS | Active. Show HN 2026-05-04 | **Camera-plus-mic meeting detection** (avoids false positives). **"Join & Transcribe" from calendar events** (Zoom, Meet, Teams, Webex). macOS Calendar "Coming Up" list. **Post-meeting hooks** (JSON on stdin). Agent CLI (`muesli-cli meetings list/get/update-notes`). Personal dictionary with fuzzy replacement. Re-summarize with a different template or model. Apple Shortcuts and Siri actions. iCloud text sync to an iPhone app. Floating recording pill. Launch at login. Onboarding wizard. Dictation |
| **OpenWhispr** [repo](https://github.com/OpenWhispr/openwhispr), 9.2k★, MIT, cross-platform | Active | Auto-detects Zoom, Teams and FaceTime. **Google, Microsoft and Apple Calendar.** **Voice fingerprinting that recognises speakers across meetings.** Notes with folders and **semantic search**. Team spaces and web sharing. Public API and MCP. CUDA and Vulkan GPU. YouTube or URL import. Native Linux. Dictation |
| **LokalBot** [repo](https://github.com/stevyhacker/LokalBot), GPLv3, macOS | Active | Recaps, decisions and action items with **citations back to the transcript or audio**. Keyword or meaning search across meetings. A **"Today" digest of open actions** where you can mark items done and trace them to source. Local drafts of follow-ups, daily briefs and weekly logs. Read-only MCP for agents. Remote model servers only after you approve their origin |
| **Platypus** [HN 47949494](https://news.ycombinator.com/item?id=47949494) (2026-04-29), Tauri/Rust | Small | Zoom and Teams auto-detection by **process inspection** (Zoom spawns `CptHost` only during a call; Teams uses `audio.mojom.AudioService`). Per-project HNSW vector search, "NotebookLM-style". Switch to API transcription when local is slow |
| **All-Ears** [HN 49091894](https://news.ycombinator.com/item?id=49091894) (2026-07-29), Linux | Small | Composable CLI daemon. **A browser extension that grabs per-participant audio streams and names from the Meet, Zoom or Teams DOM** for exact diarization. Known-word cleanup pass |
| **Scriberr** [repo](https://github.com/rishikanthc/Scriberr), 3.1k★, self-hosted web | The maintainer paused, then resumed (README status note) | **Folder watcher** that auto-processes new files (fits n8n). Chat with audio. PWA for mobile. Notes on transcripts. Diarization |
| **Vexa** [repo](https://github.com/Vexa-ai/vexa), 2.9k★, Apache-2.0 | Active | Self-hostable **bot** API for Meet, Teams and Zoom (covers meetings you don't attend). Live WebSocket transcripts. Team workspaces. Agents with the whole workspace as context. MCP |
| **Amurex** [repo](https://github.com/thepersonalaicompany/amurex), AGPL | **Stale** (last push 2025-05-27) | Real-time suggestions during meetings. **Late-join recap**. Built-in follow-up emails |
| **Omi** (BasedHardware), 13.7k★ | Active | Wearable and always-on capture, screen context |
| Others seen in 2026 | Hobnob (no source, per HN), humla, ascolto, oats, open-granola, akou (lets a terminal agent like Claude Code follow a live call), Next Notes, Aside, Utter (Mac and iPhone) | The pattern: **Markdown on disk, agent-readable (MCP/CLI), dictation bundled in** |

Market notes:
- Wispr Flow is preparing a Granola-style notetaker ([TechCrunch](https://techcrunch.com/2026/08/05/wispr-flow-is-preparing-to-launch-a-meeting-notetaker-updated-terms-suggest/), 2026-08-05).
- An HN post today (2026-10-08) claims Google has a "local first AI meeting dictation" project (Google AI Edge "Foresight"). It is unverified, because the page content did not load.

---

## 6. Synthesis

### 6.1 Killer features that make Granola and Circleback worth paying for
1. **Bot-free capture that just works.** Granola's main identity; Circleback's desktop app.
2. **Your notes plus AI enhancement in one calm notepad.** Granola.
3. **Chat and search across all meetings, with citations.** Both products; agentic since 2026.
4. **Recipes and templates** you reuse for follow-up emails, coaching and CRM fields. Granola Recipes, Circleback custom insights.
5. **Follow-through automation.** Action items with owners and dates, a task list, and CRM, Slack and Linear pushes. Circleback's main identity.
6. **Pre-meeting Briefs and People/Company pages.** Granola Briefs (2026-05); Circleback people and company pages.
7. **Calendar-driven, zero-touch capture**, plus mobile and Apple Watch for in-person meetings.
8. **MCP and API** so Claude, ChatGPT and Cursor can use meeting memory. Both products; Granola since 2026-02.
9. **Team features**: shared folders, auto-sharing rules, admin, SSO, retention.
10. **Recordings with playback and clips.** Circleback has this; it is a gap in Granola.

### 6.2 Top complaints that are openings for local or OSS tools
- **Cloud and privacy distrust:** public-link defaults, opt-out training, the July 2026 account-takeover disclosure, and "free means you're the product". **Local-first wins here.**
- **Paywalled history and export:** Granola's 30-day lock and CSV export without transcripts. **Plain files you own win here.**
- **No audio to verify against.** Local storage of audio, with playback and clip links, wins here.
- **Weak speaker labels on desktop.** Two-channel "me vs them" labelling plus local diarization wins here.
- **Silent capture failures.** Visible recording health indicators and alerts win here.
- **Consent and notification of other participants.** Built-in consent options are a differentiator.
- **Subscription fatigue.** Users explicitly prefer a **one-time or lifetime license** for local apps (Meetily disc. #525; the Hobnob author was "tired of paying a subscription for Granola").

### 6.3 Features most requested from local and OSS alternatives, ranked by signal across GitHub, HN and reviews
1. **Speaker diarization and identification**, including cheap mic=me / system=them labelling, renaming, and remembering voices across meetings.
2. **Auto-detect meetings, auto-start and auto-stop recording**, ideally driven by the calendar (Join & Transcribe).
3. **Calendar integration**: meeting titles and attendees, upcoming list, auto-naming.
4. **Custom summary templates and prompts, plus user context**: team names, glossary, vocabulary, and a notes pad fed into the summary.
5. **Chat and Q&A over one meeting and all history** (local RAG, semantic search, citations to the transcript).
6. **Export and automation:** auto-export Markdown to a chosen folder (Obsidian), PDF and DOCX, webhooks, post-meeting hooks, MCP and CLI.
7. **Editable transcript, then re-summarize**; search and replace; audio playback synced to the transcript.
8. **Remote or self-hosted transcription and LLM endpoints** (OpenAI-compatible, websocket streaming), so the GPU box can be separate.
9. **Recording-only or post-processing mode** for weak laptops.
10. **Organisation:** folders, merging meetings, a timeline, and a choice of storage location.
11. **Platform reach:** native Linux packages (AppImage, deb, Nix), mobile companion, Intel Mac.
12. **Quality of life:** dark mode, global hotkeys, floating recording indicator, tray behaviour, multitrack audio.

### 6.4 Meetily Community gaps compared with Granola, Circleback and OSS peers (quick list)
- No diarization
- No calendar
- No auto-detect
- No chat or RAG
- No custom templates UI
- No PDF, DOCX or Markdown auto-export, no webhooks, no MCP
- No transcript editing or re-summarize loop
- No notepad that steers the summary (Granola's core UX)
- No folders
- No consent or notification helpers
- No mobile app
- No native Linux package
- No Intel Mac build

Pro covers about half of these: diarization, templates, exports, auto-detect, MCP, webhooks, CLI and editing. Calendar and chat are "Soon" even in Pro.

---

## 7. Key source list (with dates)
- Meetily GitHub issues, PRs, discussions and releases, pulled 2026-10-08: https://github.com/Zackriya-Solutions/meetily
- Meetily Pro: https://meetily.ai/pro. Pricing: https://meetily.ai/pricing. Enterprise: https://meetily.ai/enterprise. Pro 1.8.2 (2026-07-22): https://meetily.ai/releases/pro/1.8.2. Pro 1.11.0 (2026-09-24): https://meetily.ai/releases/pro/1.11.0. CEO blog (2026-09-09): https://meetily.ai/blog/why-we-chose-experience-over-features
- Granola pricing: https://www.granola.ai/pricing. Updates: https://www.granola.ai/updates
- Circleback pricing: https://circleback.ai/pricing. Comparison page (vendor): https://circleback.ai/compare/granola-vs-circleback
- Product Hunt Granola reviews (4.8/5, 56 reviews): https://www.producthunt.com/products/granola/reviews. PH discussion: https://www.producthunt.com/p/granola/with-so-many-new-ai-note-takers-what-s-your-favorite-and-why
- HN threads:
  - Hyprnote Launch HN (2025-07-29, 270 pts, 180 comments): https://news.ycombinator.com/item?id=44725306
  - Circleback Launch HN (2024-03-06): https://news.ycombinator.com/item?id=39618640
  - Ask HN local-first recorder (2026-04-09): https://news.ycombinator.com/item?id=47701485
  - Muesli (2026-05-04): https://news.ycombinator.com/item?id=48011116
  - Granola Apple Watch (2026-07-28): https://news.ycombinator.com/item?id=49089026
  - All-Ears (2026-07-29): https://news.ycombinator.com/item?id=49091894
  - Platypus (2026-04-29): https://news.ycombinator.com/item?id=47949494
  - Granola action items to kanban (2026-08-29): https://news.ycombinator.com/item?id=49491815
  - Granola takeover disclosure (2026-07-22): https://news.ycombinator.com/item?id=49007641
- Reviews and blogs (several from competitors):
  - hirekai (2026-09-03): https://hirekai.ai/blog/granola-review
  - anarlog complaints (2026-08-30): https://anarlog.so/blog/granola-ai-complaints/
  - switchmytool Circleback (2026-07-09): https://www.switchmytool.com/blog/circleback-ai-review
  - happyscribe Circleback alternatives (2026-03-06): https://www.happyscribe.com/blog/circleback-alternatives-for-ai-meeting-notes
  - promptquorum Meetily (2026-09-18): https://www.promptquorum.com/power-local-llm/meetily-review
  - andrew.ooo Meetily (2026-07-06): https://andrew.ooo/posts/meetily-privacy-first-ai-meeting-assistant-review/
  - clearminutes (2026-05-01): https://clearminutes.app/alternatives/meetily
  - openwhispr compare: https://openwhispr.com/compare/meetily
  - remio Reddit summary (Jun 2026): https://www.remio.ai/post/reddit-users-are-turning-granola-into-the-new-meeting-attention-debate
  - Verge PSA summary (2026-04-02): https://www.wilsonsmedia.com/psa-anyone-with-a-link-can-view-your-granola-notes-by-default/
  - arrows.to 22-tool test: https://arrows.to/guide/top-ai-notetakers/a-side-by-side-comparison-of-22-ai-notetakers-for-sales/
- OSS repos: https://github.com/fastrepl/anarlog, https://github.com/Muesli-HQ/muesli, https://github.com/OpenWhispr/openwhispr, https://github.com/stevyhacker/LokalBot, https://github.com/rishikanthc/Scriberr, https://github.com/Vexa-ai/vexa, https://github.com/thepersonalaicompany/amurex
