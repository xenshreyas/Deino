# Deino desktop app

This folder holds the whole Deino app: the React UI (Vite, TanStack Router,
Tailwind v4) in `src/` and the Tauri 2 Rust core in `src-tauri/`. See the
[root README](../README.md) for what Deino is,
[`docs/PARITY_PLAN.md`](../docs/PARITY_PLAN.md) for the roadmap and engineering
rules, and [ADR 0003](../docs/adr/0003-vite-tanstack-router.md) for why the UI
is built this way.

## Layout

```
frontend/
├── index.html    Vite entry page (title, favicon)
├── src/          React UI: routes/ (file-based), features/, components, contexts, hooks
├── src-tauri/    Rust core: audio, transcription, storage, summaries, Tauri commands
└── public/       Static assets (logo, icons)
```

Tests are colocated with the code they cover as `*.test.ts(x)`.

## Commands

```bash
pnpm install --frozen-lockfile   # install the committed dependency set
pnpm dev                         # Vite dev server on http://localhost:3118
pnpm build                       # type-check, then build into dist/
pnpm preview                     # serve dist/ locally
pnpm test                        # Vitest (happy-dom, Testing Library)
pnpm test:watch                  # Vitest in watch mode
pnpm typecheck                   # tsc only
pnpm lint                        # ESLint
pnpm run tauri:dev               # full app in development mode (GPU auto-detected)
pnpm run tauri:build             # production bundle
```

In a plain browser (`pnpm dev`) every Tauri call fails, so the app stops at
the onboarding screen; FND-04 adds a mocked-IPC mode for browser work.

Rust-only work happens in `src-tauri/` with `cargo build` and `cargo test`.
The build script downloads a pinned FFmpeg sidecar on first build, and Tauri
expects the `llama-helper` sidecar in `src-tauri/binaries/` (see the root README).

More detail: [Building from Source](../docs/BUILDING.md),
[GPU Acceleration](../docs/GPU_ACCELERATION.md), [Architecture](../docs/architecture.md).
