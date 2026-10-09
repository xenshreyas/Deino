# Deino

Deino is a private, local-first AI meeting notepad for macOS. It records the
microphone and system audio on your Mac, transcribes locally, and turns the
transcript and your own notes into structured meeting notes. Recording,
transcription, storage and search all run on the Mac; the only data that leaves
it goes to the LLM provider and integrations you choose to connect.

This is an internal, proprietary repository. See [LICENSE](LICENSE) and
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

## Where to start

- **[`docs/PARITY_PLAN.md`](docs/PARITY_PLAN.md)**: the product definition,
  roadmap, task cards and the mandatory engineering rules (§3, §3A, §3B, §3C).
- **[`CLAUDE.md`](CLAUDE.md)**: architecture guide and conventions for agents
  and humans working in this repo.

## Stack

- One Tauri 2 app: a Rust core in `frontend/src-tauri/` and a Next.js 14 UI
  (static export) in `frontend/src/`.
- SQLite through sqlx, with migrations in `frontend/src-tauri/migrations/`.
- Local transcription (Parakeet via ONNX Runtime, Whisper via whisper.cpp).
- Sidecars: `llama-helper` (built-in local LLM, this workspace) and FFmpeg
  (downloaded and SHA-256 verified by the build script).

## Requirements

- macOS 14.2 or later on Apple Silicon (primary target). Windows and Linux
  must still compile.
- Rust stable, Node.js 20+, pnpm 9, Xcode Command Line Tools and CMake.

## Run in development

```bash
# 1. Build the llama-helper sidecar once and stage it where Tauri expects it
cargo build --release -p llama-helper --features metal
mkdir -p frontend/src-tauri/binaries
cp target/release/llama-helper \
   "frontend/src-tauri/binaries/llama-helper-$(rustc -vV | sed -n 's/^host: //p')"

# 2. Install UI dependencies and start the app
cd frontend
pnpm install --frozen-lockfile
pnpm run tauri:dev
```

The first Rust build downloads the pinned FFmpeg sidecar into
`frontend/src-tauri/binaries/`.

## Checks

```bash
cd frontend && pnpm run build && bun test   # UI build and tests
cd frontend/src-tauri && cargo test         # Rust tests
```

More detail: [Building from source](docs/BUILDING.md),
[GPU acceleration](docs/GPU_ACCELERATION.md), [Architecture](docs/architecture.md).
