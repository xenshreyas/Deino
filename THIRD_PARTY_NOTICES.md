# Third-Party Notices

Deino is derived from Meetily (https://github.com/Zackriya-Solutions/meetily), v0.4.1,
which is licensed under the MIT License reproduced below. This notice must be retained
in all copies or substantial portions of the original Meetily code.

---

MIT License

Copyright (c) 2024 Zackriya Solutions

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

---

## Anarlog

Source: https://github.com/fastrepl/anarlog (formerly Hyprnote), MIT License.

Adapted in Deino:

- `frontend/src/test-setup.ts`: the Vitest `__TAURI_INTERNALS__` /
  `__TAURI_EVENT_PLUGIN_INTERNALS__` shim, from `apps/desktop/src/test-setup.ts`.
- `frontend/vite.config.ts`: the TanStack Router plugin setup and the Tauri
  dev-server block, from `apps/desktop/vite.config.ts`.
- `frontend/src/hooks/useMountEffect.ts`: the `useMountEffect` hook, from
  `packages/ui/src/hooks/use-mount-effect.tsx`.
- `crates/deino-db/src/change_notifier.rs`: commit-time table change signals
  from SQLite update/commit/rollback hooks, from `crates/db-change`.
- `frontend/src-tauri/src/bindings.rs`: generating tauri-specta bindings from a
  test, from `plugins/settings/src/lib.rs`.
- `docs/TESTING.md`: the "write a test when / do not write a test for"
  guidance, adapted from `.agents/skills/testing/SKILL.md`.
- `crates/deino-doc/`: the TipTap/ProseMirror JSON ↔ Markdown conversion and
  the document validation, from `crates/tiptap` (`from_md.rs`, `to_ast.rs`,
  `validate.rs` and their tests), ported to a typed model and extended with
  Deino's provenance, citation and mention nodes (NOT-01).
- `frontend/public/theme-boot.js`: applying the saved theme before the first
  paint, after `apps/desktop/public/theme-boot.js`; the System / Light / Dark
  picker with a diagonally split System preview
  (`frontend/src/features/appearance/components/theme-picker.tsx`) follows
  `apps/desktop/src/settings/appearance/theme.tsx`.
- `crates/deino-recording/` and `crates/deino-audio/src/capture/` (CAP-02):
  skipping echo handling when the output is headphones-only
  (`echo.rs`, after `crates/audio`'s `headphone_only_output`), the 1 s
  device-event debounce (`capture/supervisor.rs`), the stable aggregate-device
  UID for the process tap (`capture/macos/tap.rs`), and earshot with a
  hangover as the live per-lane VAD (`vad.rs`), after `crates/audio` and
  `crates/vad/src/earshot.rs`.
- `crates/deino-llm/src/prompts.rs` and `crates/deino-llm/prompts/`: built-in
  prompts as askama `.md.jinja` templates with inline insta snapshots (the
  `prompt_snapshot!` macro), after `crates/template-app` and
  `crates/askama-utils/src/lib.rs` (`tpl_snapshot!`).
- `crates/deino-llm/src/user_templates.rs`: strict-undefined minijinja user
  templates validated against a variable allowlist, after
  `crates/template-app/src/enhance.rs`.
- `crates/deino-llm/tests/llm_eval/`: the libtest-mimic eval harness
  (contract, smoke and live trials) and its `Expectation` enum, after
  `crates/template-eval/src/eval/{expectations.rs,harness.rs}`.
- `crates/deino-diarize/` (CAP-03): the speaker diarizer, ported from
  `crates/pyannote-local` (`pipeline.rs`, `segmentation.rs`,
  `clustering.rs`, `metrics.rs`), `crates/embedding/src/onnx/mod.rs` and
  `crates/onnx/src/lib.rs`.
- `crates/deino-core/src/voiceprints/{matching,spans}.rs` (CAP-03, moved
  from `deino-diarize` by CAP-04): the voiceprint matching primitives and
  enrolment spans from `crates/voiceprint/src/{lib,matching}.rs`.
- `crates/deino-core/src/diarization/plan.rs` and `assign.rs`: channel
  planning (the in-person shared-microphone rule) from
  `crates/listener2-core/src/batch/diarize.rs`, and word-to-speaker
  assignment from `pyannote-local::assign_words`.
- `crates/deino-detect/` (CAP-06): the deny-list categories
  (`classify/tables.rs`, from `plugins/detect/src/policy.rs`), the 15 s
  microphone hold and 10-minute cooldown (`policy.rs`, from
  `plugins/detect/src/mic_usage_tracker.rs`), resolution to the outermost
  `.app` bundle and the FaceTime / iPhone call-daemon relabelling
  (`attribution.rs`, from `crates/detect/src/list/macos.rs`), and the
  browser bundle list (from `apps/desktop/src/stt/meeting-apps.ts`).

MIT License

Copyright (c) 2023-present Fastrepl, Inc.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

---

## Handy

Source: https://github.com/cjpais/Handy, MIT License.

Adapted in Deino:

- `crates/deino-llm/src/providers/openai_compat.rs`: retrying an
  OpenAI-compatible request without its optional fields (`response_format:
  json_schema` and friends) when the server answers 400 or 422, after
  `src-tauri/src/llm_client.rs`.

MIT License

Copyright (c) 2025 CJ Pais

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

---

## tauri-specta

Source: https://github.com/specta-rs/tauri-specta, MIT License.

Adapted in Deino:

- `frontend/src-tauri/src/features/*.rs`: the per-plugin `tauri_specta::Builder`
  pattern from `examples/custom-plugin/plugin/src/lib.rs`, and the typed event
  pattern from `examples/app`.

MIT License

Copyright (c) 2022 Oscar Beaumont

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

---

## Muesli

Source: https://github.com/Muesli-HQ/muesli, MIT License.

Adapted in Deino:

- `docs/REVIEW.md`: the severity scale and review-priority structure, from
  `REVIEW.md`.
- `crates/deino-detect/` (CAP-06), ported from Swift: the meeting-app table
  with its weak, full-duplex-only apps and the candidate priority order
  (`classify/tables.rs`, `resolver.rs`, from `MeetingCandidateResolver.swift`),
  the meeting-URL normaliser (`url.rs`, from `MeetingURLNormalizer`), the
  candidate stability rule (`policy.rs`, from
  `MeetingPromptStateMachine.swift`), and the CoreMediaIO camera observer
  (`macos/cmio.rs`, `macos/camera.rs`, from `CameraActivityMonitor.swift`).
- `crates/deino-audio/src/capture/` (CAP-02): the system-audio watchdog's
  recovery episodes (2 s / 5 s backoff, at most 4 attempts, 15 s cooldown,
  two healthy ticks to clear; `watchdog.rs`), reimplemented from
  `MeetingSystemAudioWatchdog.swift`.

MIT License

Copyright (c) 2026 Pranav Hari

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

---

## shadcn/ui

Source: https://github.com/shadcn-ui/ui, MIT License.

Adapted in Deino:

- `frontend/src/ui/`: the Radix-based primitives (button, dialog, popover,
  dropdown menu, select, tabs, tooltip, command, scroll area, accordion,
  alert, switch, label, separator, progress, button group), restyled to
  Deino's design tokens.

MIT License

Copyright (c) 2023 shadcn

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.


---

## Handy

Source: https://github.com/cjpais/Handy, MIT License.

Ported in Deino (CAP-01), restructured to Deino's limits and typed errors:

- `crates/deino-stt/src/protocol.rs`, `protocol/frame.rs`: the framed worker
  protocol, from `src-tauri/src/engine_supervisor/protocol.rs`.
- `crates/deino-stt/src/worker.rs`, `worker/`: the worker process (protocol on
  the real stdout, `_exit` on stdin EOF, the stderr logger), from
  `engine_supervisor/worker.rs`.
- `crates/deino-stt/src/supervisor.rs`, `supervisor/`: the supervisor (owner
  thread, deadlines scaled to the audio, GPU→CPU fallback, cancel by kill,
  lost streams), from `engine_supervisor/supervisor.rs`.
- `crates/deino-stt/src/engine/scripted.rs`: worker fault injection, from the
  `HANDY_WORKER_FAULT` hooks in `engine_supervisor/worker.rs`.
- `crates/deino-stt/src/perf.rs`: `StreamPerf` metrics, from
  `managers/transcription.rs`.
- `crates/deino-core/src/models/download.rs`: the resumable download's
  robustness rules, from `managers/model/download.rs`.
- `crates/deino-core/src/models/catalog.json`: the Parakeet Unified EN 0.6B
  entry (revision, sizes, SHA-256 digests), from `src/catalog/catalog.json`.

MIT License

Copyright (c) 2025 CJ Pais

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

---

## Inter (font)

Source: https://github.com/rsms/inter, bundled through `@fontsource-variable/inter`.

Copyright 2016 The Inter Project Authors (https://github.com/rsms/inter).

This Font Software is licensed under the SIL Open Font License, Version 1.1.
The full licence ships with the package (`@fontsource-variable/inter/LICENSE`)
and is available with a FAQ at https://openfontlicense.org. The font is used
unmodified and is not sold on its own.

The PDF export (EXP-01) embeds four static instances of the same font in
the app binary: `crates/deino-export/fonts/Inter-{Regular,SemiBold,Italic,SemiBoldItalic}.ttf`,
instantiated at weights 400 and 600 from the latin and latin-ext subsets of
`@fontsource-variable/inter` 5.3 with fontTools, with the family name
"Inter" kept (the OFL has no Reserved Font Name for Inter). The licence is
next to them in `crates/deino-export/fonts/OFL.txt`.

---

## transcribe.cpp

Source: https://github.com/handy-computer/transcribe.cpp (the
`transcribe-cpp` and `transcribe-cpp-sys` crates, 0.3.1), MIT License,
Copyright (c) 2026 The transcribe.cpp authors. Linked as a dependency of
`crates/deino-stt` (it bundles ggml, MIT License, Copyright (c) 2023-2024 The
ggml authors).

---

## Parakeet Unified EN 0.6B (model weights)

Deino downloads `nvidia/parakeet-unified-en-0.6b` as GGUF files converted by
handy-computer (`handy-computer/parakeet-unified-en-0.6b-gguf`, pinned
revision `7e948f21b7bdbac698d3318db9d350f1096f3b6c`). The weights are not
bundled with the app; they are fetched on first use.

Licensed by NVIDIA Corporation under the NVIDIA Open Model License.
https://www.nvidia.com/en-us/agreements/enterprise-software/nvidia-open-model-license/

---

## OpenWhispr

Source: https://github.com/OpenWhispr/openwhispr, MIT License.

Adapted in Deino: `crates/deino-diarize/src/prune.rs` (phantom-speaker pruning
and collapse detection, from `src/helpers/diarizationPolicy.js`) and
`crates/deino-diarize/src/online.rs` (the threshold-plus-margin live speaker
rules, from `src/helpers/liveSpeakerIdentifier.js`). `crates/deino-detect/`
(CAP-06) ports the PID-scoped microphone listener from
`resources/macos-mic-listener.swift`: listeners on the process list and on
each process object, a reconcile that skips objects that vanished mid-read,
and ignoring `com.apple.CoreSpeech` (`macos/processes.rs`, `reader.rs`).
`crates/deino-recording/` (CAP-02) ports the meeting-mode bleed holdback from
`meetingMicHoldback.js`, with its text-match test (`bleed.rs`,
`bleed/overlap.rs`); the
capture supervisor's sleep detection by a wall-clock tick gap
(`crates/deino-audio/src/capture/supervisor/actor.rs`) follows its
recording watchdog.

Copyright (c) 2024 OpenWhispr Team. Licensed under the MIT License (the
same terms as the Anarlog notice above).

---

## knf-rs and kaldi-native-fbank

`crates/third_party/knf-rs/` vendors knf-rs 0.3.2
(https://github.com/thewh1teagle/pyannote-rs, revision `e23bd29`, MIT License,
`LICENSE-knf-rs`), which wraps kaldi-native-fbank 1.20.1
(https://github.com/csukuangfj/kaldi-native-fbank, Apache License 2.0,
`LICENSE-kaldi-native-fbank`). Deino's changes: the C++ is compiled with `cc`
instead of CMake and bindgen, and the Rust wrapper returns typed values and
errors; the C++ sources are unchanged.

---

## pyannote segmentation-3.0 (model weights)

Downloaded on demand (not bundled) from
https://huggingface.co/onnx-community/pyannote-segmentation-3.0 (revision
`733a93b`, file `onnx/model.onnx`, SHA-256 `057ee564…ea25`). Original model:
https://huggingface.co/pyannote/segmentation-3.0, MIT License,
Copyright (c) 2023 CNRS (Hervé Bredin and contributors).

---

## Nomic Embed Text v1.5 (model weights)

Downloaded on demand (not bundled) from
https://huggingface.co/nomic-ai/nomic-embed-text-v1.5-GGUF (revision
`0188c9b`, file `nomic-embed-text-v1.5.Q8_0.gguf`, SHA-256 `3e243421…c3b7`),
a GGUF conversion of https://huggingface.co/nomic-ai/nomic-embed-text-v1.5,
licensed under the Apache License 2.0, Copyright Nomic AI. Deino uses it
unmodified, on the Mac, to embed meeting text for semantic search.

---

## WeSpeaker ResNet34-LM (model weights)

Downloaded on demand (not bundled) from
https://huggingface.co/Wespeaker/wespeaker-voxceleb-resnet34-LM (revision
`f0c48c2`, file `voxceleb_resnet34_LM.onnx`, SHA-256 `7bb2f06e…c068`).

"wespeaker-voxceleb-resnet34-LM" by the WeSpeaker authors (Hongji Wang,
Chengdong Liang, Shuai Wang, Zhengyang Chen, Binbin Zhang, Xu Xiang, Yanlei
Deng and Yanmin Qian), trained on VoxCeleb, is licensed under the Creative
Commons Attribution 4.0 International License (CC BY 4.0,
https://creativecommons.org/licenses/by/4.0/). Deino uses it unmodified to
compute speaker embeddings. Paper: "WeSpeaker: A Research and Production
oriented Speaker Embedding Learning Toolkit", ICASSP 2023.

---

## Vexa (Google Meet speaker classes)

Source: https://github.com/Vexa-ai/vexa (v0.12.18, `core/meetings/modules/gmeet-capture/`),
Apache License 2.0.

`crates/deino-speaker-tags/src/adapters/meet.rs` (CAP-05) uses the CSS class
names Vexa's Meet speaker detection lists for a speaking tile (`Oaajhc`,
`HX2H7`, `wEsLMd`, `OgVli` and the semantic `speaking`, `active-speaker`,
`speaker-active`, `speaking-indicator`) and its observation that a tile's
name is a `span.notranslate`. No Vexa code is included; Deino reads these
classes through Chromium's Accessibility `AXDOMClassList` instead of the DOM.
