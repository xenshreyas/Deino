# Reference guide: what to study before building each card

Deino benchmarks itself against the best open-source projects in each area (see `PARITY_PLAN.md` §3 rule 14).
- The projects are cloned locally at `~/Documents/GitHub/deino-references/`.
- The studies below cite exact files and line ranges in those clones. **Read the listed sections before designing your card.**
- If you find something better than what a study recommends, take it, and record why in your PR or an ADR.

## Licences

| Repo | Licence | Use |
|---|---|---|
| Handy | MIT | Copy with attribution |
| anarlog (ex-Hyprnote) | MIT, **except `enterprise/**`**, which is commercial and must never be touched | Copy with attribution |
| spacedrive | Apache-2.0 | Copy with attribution |
| muesli | MIT (vendored Qwen3ASR is Apache-2.0) | Copy or port the logic with attribution |
| openwhispr | MIT (the mediaremote adapter is BSD-3) | Port the logic with attribution |
| sherpa-onnx | Apache-2.0 | Use as a dependency, or port |
| tauri plugins-workspace, tauri-specta, shadcn/ui, TanStack | MIT / Apache-2.0 | Copy |
| **Cap** | AGPL-family | **Reference only.** Learn the pattern, write your own code. |
| **zed** | GPL/AGPL-family | **Reference only.** |

**Model files** carry their own licences. Check each one, and record it in `THIRD_PARTY_NOTICES.md`:
- WeSpeaker embeddings: probably CC-BY-4.0.
- Parakeet: CC-BY-4.0.
- DTLN-aec and Silero: MIT.

**LAME is LGPL.** Do not link it.

## Studies (in `docs/references/`)

| File | Covers |
|---|---|
| `architecture.md` | Workspace and crate layout, inline Tauri plugins and the ACL, tauri-specta, errors, DI, sqlx, the LLM gateway, the Vite+TanStack frontend, tooling and lint configs, testing, docs, and Zed's idioms. **Sections (a)–(c) give the target layout, per-card recommendations and the config files to copy.** |
| `audio-baseline-audit.md` | What is wrong with Deino's current audio code (real-time-safety violations, mixing before STT, 30 s checkpoints). |
| `audio-capture.md` | The process tap, real-time-safe rings, device changes, sleep/wake, Bluetooth/HFP, track alignment, crash-safe fragmented recording, meeting detection, auto-stop, health. **§9 is the target audio architecture; §10 has per-card recommendations.** |
| `audio-stt.md` | transcribe-cpp 0.3.1 streaming (Handy), two-lane memory, timestamps, model management, diarization (anarlog `pyannote-local` vs sherpa-onnx), voiceprints, echo/bleed, performance measurement. **§0 lists the findings that change the plan.** |
| `audio-handy.md` | A condensed map of Handy's engine supervisor, streaming lifecycle, downloader and capture. |
| `audio-anarlog.md` | A condensed map of anarlog's tap, AEC (DTLN), joiner/aligner, detection, chunked recorder and diarization. |
| `audio-muesli-openwhispr.md` | The detection resolver (Muesli), auto-end controller and mic holdback (OpenWhispr), the live speaker ID and EMA voiceprints. |
| `audio-diarization.md` | The sherpa-onnx Rust API, pipeline facts and scaling limits. |
| `audio-cap-zed.md` | Reference-only patterns: sample-count timeline, gap tracker, epoch anchoring, fragmented M4A, recovery, health bus, Mach clock, device listeners, WebRTC AEC. |
| `ui.md` | The design token spec (exact light and dark hex values with WCAG ratios), component specs, screen wireframes and reuse list. **§6 is the FND-10 checklist.** |

## Card → reading list

| Card | Read first |
|---|---|
| FND-02 | `architecture.md` §1 and (b) FND-02 |
| FND-03 | `architecture.md` §8 and (b) FND-03, (c) |
| FND-04 | `architecture.md` §9 and (b) FND-04; `audio-anarlog.md` §9 (audio-mock) |
| FND-05 | `audio-capture.md` §2, §4, §5, §9, §10 FND-05; `audio-anarlog.md` §8; `audio-cap-zed.md` §1–4 |
| FND-06 | `architecture.md` (b) FND-06 |
| FND-07 | `architecture.md` §5 and (b) FND-07 |
| FND-08 | `architecture.md` (b) FND-08 (spacedrive job contract) |
| FND-09 | `architecture.md` §6 and (b) FND-09 |
| FND-10a / FND-10 | `architecture.md` §7 and (b) FND-10; `ui.md` (all) |
| FND-12 | `architecture.md` (b) FND-12 |
| FND-13 | `architecture.md` §2, §3, §8, §10, §11 and (b) FND-13, (c) |
| CAP-01 | `audio-stt.md` §0–2 and the CAP-01 card notes; `audio-handy.md` |
| CAP-02 | `audio-stt.md` §5; `audio-capture.md` §4; `audio-anarlog.md` §3–4; `audio-muesli-openwhispr.md` (echo/bleed); `audio-cap-zed.md` §9 |
| CAP-03 / CAP-04 | `audio-stt.md` §3–4; `audio-anarlog.md` §6; `audio-diarization.md`; `audio-muesli-openwhispr.md` (live ID, EMA) |
| CAP-06 | `audio-capture.md` §6 and §10; `audio-muesli-openwhispr.md` (Muesli detection); `audio-anarlog.md` §7 |
| CAP-07 | `audio-capture.md` §7; `audio-muesli-openwhispr.md` (OpenWhispr auto-end, Muesli auto-stop) |
| CAP-08 | `audio-stt.md` (CAP-08 recommendations) |
| CAP-09 | `audio-capture.md` §10 CAP-09 (`cidre::sc`) |
| CAP-10 | `audio-capture.md` §7; `audio-muesli-openwhispr.md` (mic health, coverage audit); `audio-cap-zed.md` §5; `ui.md` (record pill) |
| NOT-01 / NOT-02 | `architecture.md` §7 (TipTap and `deino-doc`), §6 (prompt templates); `ui.md` (notes view, AI vs user text) |
| TRN-01 / TRN-03 | `ui.md` (transcript bubble, player) |
| All UI cards | `ui.md` §1–3 |
