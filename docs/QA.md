# Owner QA checklist

Manual checks only a person on a real Mac can do. Each line is a step, what
"pass" looks like, and the card ID (the full text is in the card's row in
`docs/PARITY_PLAN.md` §7.2). Tick a box when it passes; when it fails, note
the card ID and what you saw. The riskiest items (real calls, permissions,
deleting data) come first in each group.

## Before you start

- Use a fresh data folder or a backup of your real one; never test encryption or retention on the only copy.
- Add an Anthropic API key in Settings → AI (notes, evals and screen descriptions need it).
- Have Zoom and Google Meet test calls ready (a second device or person), and Teams if you use it.
- Grant Deino Accessibility and Screen Recording in System Settings → Privacy & Security, plus Microphone and Calendars.
- Use the signed or bundled app (`pnpm run tauri:build`, copy to `/Applications`), not `dev:mock`; mock cannot show macOS prompts.

## Recording & capture

- [ ] **Call detection.** Run the list in `crates/deino-detect/README.md`: Zoom, Meet in Chrome, Slack huddle, the ignore list, auto-start. Pass: Deino offers or starts a recording for each real call and ignores the ignored apps. `CAP-06`
- [ ] **A call that ends.** Finish one real call, then try Keep recording, sleeping the lid, and back-to-back events with different invitees. Pass: microphone released, then "Ending…", then stop; Keep recording continues; each event gets its own invitees. `CAP-07`
- [ ] **Hardware changes.** Connect AirPods mid-call, unplug a USB microphone, sleep and wake during a recording, and see the first-run permission prompt. Pass: recording survives each, no gap you can hear. `CAP-02`
- [ ] **Recording health.** Unplug the microphone and kill an STT worker while recording. Pass: a warning within 10 s. `CAP-10`
- [ ] **Pill focus.** Type in Zoom or a browser while hovering and clicking the pill. Pass: focus never leaves the app you type in. `CAP-10`
- [ ] **Pill and menu bar.** Drag the pill and relaunch; switch microphone and call audio mid-recording; use the menu bar's countdown, "Xm left", Join, Record and Open. Pass: position kept, switches work, all items act. `CAP-10`
- [ ] **Join & record.** Use Join & record on a Zoom and a Meet event; check the reminder panel's placement and focus; open an event with `ical://ekevent/`. Pass: capture starts within 2 s; the event opens in Calendar. `ORG-02`
- [ ] **Calendar sync.** Turn sync on in Settings → Calendar, accept the Calendars prompt, add an event in Calendar.app, sleep and wake, then deny access in System Settings. Pass: the event shows within seconds; after denial, an explainer and an "Open System Settings" link. `ORG-01`
- [ ] **Event linking.** Start a recording a few minutes before a calendar event; re-link mid-recording; unlink; open a recurring meeting and follow "Previous". Pass: the header chip, attendees and title match the event; the links work. `ORG-03`
- [ ] **Playback in the bundled app.** Play `audio.mp4` and `mic.m4a`, seek, and use the download save sheet. Pass: sound plays, seeking is instant, the file saves. `TRN-03`
- [ ] **CPU on real devices.** Repeat the CPU measurement with a release build on a quiet machine, record-only and live. Pass: record the figures and compare them with the mock run (6.0 % record-only, 10.1 % live, of one core). `CAP-08`

## Calls (speaker names, consent notice, screen capture)

- [ ] **Consent chat notice.** Post it on real Zoom and Meet test calls (and Teams). Pass: it posts once; nothing posts while you type or into a draft; no repeat after pause/resume. `CAP-13`
- [ ] **Zoom recipient.** Send a direct message in Zoom, then trigger the notice. Pass: the notice goes to Everyone (if not, decide whether to select "Everyone" first). `CAP-13`
- [ ] **Speaker names.** On real Zoom, Meet and Teams calls, check the transcript's speaker names. Pass: at least 90 % of segments named right for people who talk more than 3 s. `CAP-05`
- [ ] **Speaker lag and CPU.** On those calls, check turn starts line up (Zoom's indicator lags about 0.5 to 1 s) and note Deino's and Chrome's CPU. Pass: starts align; CPU is not high. `CAP-05`
- [ ] **Real window captures.** Capture real chat and call windows as `fixtures/README.md` says and replace the modelled dumps; run the tests. Pass: tests pass, or `ChatVocabulary` is adjusted until they do. `CAP-13`, `CAP-05`
- [ ] **Full call with screen capture.** Turn on "Capture detected meetings" and run a detected Zoom or Teams call. Pass: video plays in the player, a clip has sound, snapshots appear on disk, minimising the window shows "Paused". `CAP-09`
- [ ] **Call with slides.** Share slides, then check the notes. Pass: OCR text appears on the rows, the enhanced notes use it; "Describe snapshots with Claude" sends pictures only when on; the picker lists screens and windows and a manual start captures; the reminder shows on the first three captures. `CAP-09`

## Privacy (Touch ID lock, encryption, retention)

- [ ] **Encrypt a library.** Back it up first. Encrypt a real library, relaunch, check meetings, search and the `deino` CLI, then decrypt. Pass: everything still opens and the Keychain item is gone after decrypting. `PRV-02`
- [ ] **Retention sweep.** On a copy of the data folder with a short period, run with `RUST_LOG=deino_lib=info,deino_core::retention=info`. Pass: only meetings older than the period are removed, and the log says so. `PRV-01`
- [ ] **Touch ID lock.** Unlock with Touch ID, the password fallback, and Cancel; lock by idle and by wake/screen lock; keep a recording running behind the lock screen. Pass: each path behaves, and the recording does not stop. `PRV-02`
- [ ] **Keychain contract tests.** Run `cargo test -p deino-secrets -- --ignored` (login keychain) and `deino` CLI reads of the app key on a signed build. Pass: tests pass; at most one prompt per signature. `PRV-02`, `INT-00`

## Onboarding

- [ ] **Permission prompts.** On a fresh data folder, go through each prompt (microphone, system audio, Accessibility, notifications, Calendar), every "Open System Settings" link, the 20-second test recording, and the sample meeting (open, delete, import again). Pass: each prompt appears once, each link opens the right pane, the test recording plays back. `FND-11`
- [ ] **Time to first recording.** Time a fresh install to its first recording. Pass: under 3 minutes on a good connection (record the time). `FND-11`
- [ ] **Settings pages.** Open every page in Settings and search for each. Pass: same order and labels as before, and search finds each page. `REG-02`

## Imports

- [ ] **Importers with real keys.** For each provider paste a real key, Check, Import, Stop midway, re-import, then search and ask about an imported meeting. Pass: re-import adds nothing; the egress log shows only the provider host; offline mode refuses Check. `CAP-12`
- [ ] **Two-hour import.** Drop a long MP4 into a watched folder (fresh data folder). Pass: the meeting reaches `ready` with notes, the video plays, "Move to processed" moves the file. `CAP-11`
- [ ] **Image paste and drag.** Press ⌘⇧V with an image on the clipboard, and drag an image over the window. Pass: the image pastes (WKWebView may refuse the clipboard); no audio-import overlay appears for an image. `NOT-01`

## Automations, webhooks, API & links

- [ ] **Script step.** Run an automation with a real script (payload on stdin, cwd the meeting folder), plus the export step's folder dialog, a notification step, and a tag trigger on an upcoming meeting. Pass: the script gets the payload and is killed after 60 s with its children; dialog and notification appear; the tag arms the meeting. `AUT-01`
- [ ] **Hotkeys and links.** Follow the numbered list in the DEV-05 row (Safari in front, ⌃⌥⌘R/N/J, closed window, Change to ⌃⌥K, `deino://` links from Terminal, launch from a link, App lock, a Raycast script). Pass: each behaves as that row says, and ⌘⇧4 is refused with the old shortcut kept. `DEV-05`
- [ ] **Local API.** Turn the API on, make a read-only and a read-write key, then `curl -H "Authorization: Bearer <key>" http://127.0.0.1:47810/v1/meetings?limit=2`, follow the `Link` header, `POST` a transcript, lock Deino. Pass: reads work, writes need the right key, the locked app returns `423`. Take light and dark screenshots of the page. `DEV-03`
- [ ] **Webhooks.** Add a Zapier Catch Hook and a Make Custom webhook; send a test from Settings → Developer → Webhooks. Pass: both receive it, and Make's `x-signature` check passes with `sha256(body; "hex"; secret)`. `DEV-01`
- [ ] **Claude Code.** With a signed-in `claude`, run `claude -p "list my last 3 meetings and their action items"` (`docs/setup/claude-code.md`). Pass: it lists them via the `mcp__deino__*` tools. `DEV-02`
- [ ] **Semantic search from the CLI.** Run `deino search --json` with the embedding model downloaded. Pass: `"mode": "hybrid"`, and no `llama-helper` outlives `deino` or a closed Claude session. `DEV-02`

## AI notes

- [ ] **Action-items eval.** Run `DEINO_EVAL=1 ANTHROPIC_API_KEY=… pnpm eval extract.action_items`. Pass: precision at least 0.8 and recall at least 0.5; record the score. `AI-05`
- [ ] **Insights eval.** Run `DEINO_EVAL=1 ANTHROPIC_API_KEY=… pnpm eval extract.insights`. Pass: Decisions recall at least 0.6 and at least 0.9 attributed over the eight fixtures; record the score. `AI-06`
