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
- `frontend/public/theme-boot.js`: applying the saved theme before the first
  paint, after `apps/desktop/public/theme-boot.js`; the System / Light / Dark
  picker with a diagonally split System preview
  (`frontend/src/features/appearance/components/theme-picker.tsx`) follows
  `apps/desktop/src/settings/appearance/theme.tsx`.

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
