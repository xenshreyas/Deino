# Deino desktop app

This folder holds the whole Deino app: the Next.js 14 UI (statically exported)
in `src/` and the Tauri 2 Rust core in `src-tauri/`. See the
[root README](../README.md) for what Deino is and
[`docs/PARITY_PLAN.md`](../docs/PARITY_PLAN.md) for the roadmap and engineering
rules.

## Layout

```
frontend/
├── src/          Next.js UI (app routes, components, contexts, hooks)
├── src-tauri/    Rust core: audio, transcription, storage, summaries, Tauri commands
├── tests/        bun:test suites for hooks and libs
└── public/       Static assets (logo, icons)
```

## Commands

```bash
pnpm install --frozen-lockfile   # install the committed dependency set
pnpm run dev                     # Next.js dev server on http://localhost:3118
pnpm run build                   # static export into out/
pnpm run tauri:dev               # full app in development mode (GPU auto-detected)
pnpm run tauri:build             # production bundle
bun test                         # frontend tests
```

Rust-only work happens in `src-tauri/` with `cargo build` and `cargo test`.
The build script downloads a pinned FFmpeg sidecar on first build, and Tauri
expects the `llama-helper` sidecar in `src-tauri/binaries/` (see the root README).

More detail: [Building from Source](../docs/BUILDING.md),
[GPU Acceleration](../docs/GPU_ACCELERATION.md), [Architecture](../docs/architecture.md).
