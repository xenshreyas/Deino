# 0004. Build Deino as one Tauri app with a Rust core

- **Status:** Accepted
- **Date:** 2026-10-08
- **Card:** FND-13 (records the PARITY_PLAN §2/§4.0 baseline)

## Context

Deino is a private meeting notepad for one person on macOS. It captures the
microphone and system audio (Core Audio process taps, macOS 14.2+), runs
speech-to-text locally (Parakeet over ONNX Runtime, Whisper over whisper.cpp),
stores everything in SQLite and calls an LLM for notes. The inherited app
(Meetily v0.4.1) is already a Tauri 2 app with a Rust core. Every reference
project we benchmark (anarlog, Handy, Cap, spacedrive) is Tauri plus Rust.

## Decision

Keep one Tauri 2 application:

- **Rust owns the work and the data:** capture, transcription, storage, the
  post-meeting pipeline, LLM and network calls, OS integration.
- **The UI is a React app in the system WKWebView** that renders and sends
  intents through typed IPC (ADR 0007). It never owns a copy of server state
  outside TanStack Query (ADR 0008).
- **Heavy native work that can crash or hang runs in child processes:** the
  llama.cpp sidecar today; transcribe-cpp workers (CAP-01).
- Tauri-free logic lives in `crates/` (ADR 0006) so a CLI, MCP server or REST
  API (DEV-02/03/04) can reuse it without a webview.

## Alternatives

- **Native Swift/AppKit (muesli):** best macOS integration, but discards the
  working Rust audio and STT stack and the web UI, and ties the logic to Apple
  platforms.
- **Electron:** a heavier runtime and a Node main process; the audio and STT
  code would still be native modules.
- **A local server with a browser UI:** an extra process to supervise, and no
  tray, notifications or permission prompts without a shell app anyway.

## Consequences

- One binary, a small runtime and direct access to Core Audio, Metal and
  CoreML from Rust.
- IPC is a boundary to design (ADR 0007), and WKWebView sets the browser
  baseline (Safari 17).
- Windows and Linux still compile, so new macOS-only code needs
  `#[cfg(target_os = "macos")]` and stubs (§3 rule 11).
