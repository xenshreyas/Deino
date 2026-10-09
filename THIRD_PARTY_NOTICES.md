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

## Inter (font)

Source: https://github.com/rsms/inter, bundled through `@fontsource-variable/inter`.

Copyright 2016 The Inter Project Authors (https://github.com/rsms/inter).

This Font Software is licensed under the SIL Open Font License, Version 1.1.
The full licence ships with the package (`@fontsource-variable/inter/LICENSE`)
and is available with a FAQ at https://openfontlicense.org. The font is used
unmodified and is not sold on its own.

