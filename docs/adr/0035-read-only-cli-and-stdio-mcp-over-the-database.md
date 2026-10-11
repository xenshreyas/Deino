# 0035. Serve Claude from one read-only `deino` binary over stdio MCP

- **Status:** Accepted
- **Date:** 2026-10-11
- **Card:** DEV-02 (local MCP server) and DEV-04 (CLI), re-scoped by D12

## Context

The owner wants Claude Code to read their meetings: "expose the notes and
transcripts via CLI and MCP so I can hook it up to Claude Code". The DEV-02
card planned a Streamable HTTP server inside the app with a bearer token
and a `deino-mcp` stdio shim proxying to it, plus write tools and
`ask_deino`; DEV-04 planned a CLI over DEV-03's REST API. Both waited on
AI-02, DEV-03 and ORG-07. D12 (PARITY_PLAN §8) promotes read access for
Claude Code and drops those dependencies.

Constraints: the app may not be running when Claude asks; the database is
the app's (WAL, one writer, `crates/deino-db`); nothing may ever corrupt or
migrate it from outside; meeting content must not leave the Mac except to
the Claude client the user connected; every read must match what the app
shows (names, the newest enhanced notes, deleted transcript lines hidden).

## Decision

1. **One binary, two faces.** `crates/deino-cli` builds one executable.
   `deino mcp` is an MCP server over stdio (the official Rust SDK, `rmcp`
   3.5.1, Apache-2.0); every other subcommand is a `clap` CLI with `--json`.
   Each read is one `Request` type (clap flags, serde arguments, a schemars
   input schema, a typed `Output` with an output schema, compact Markdown),
   so the CLI and the MCP tools cannot drift. Tools return Markdown text and
   the same data as `structuredContent`; failures are tool errors carrying
   `{"error": {"code", "message"}}` with the app's `ErrorCode`s.
2. **Read the database directly, read-only.** `Database::open_read_only`
   opens `deino.sqlite` with `SQLITE_OPEN_READONLY` (`mode=ro`), `query_only`,
   WAL and the app's 5 s busy timeout. It never creates the file and never
   runs a migration: it compares the applied migrations with the binary's
   embedded `MIGRATOR` (`SchemaCheck`) and refuses a newer database ("reinstall
   the CLI from the updated app") or an older one ("open the app once"). The
   path is `--db`, then `DEINO_DB`, then the app's own
   (`dirs::data_dir()/com.xenshreyas.deino/deino.sqlite`, what Tauri's
   `app_data_dir` resolves to; a test pins the identifier to
   `tauri.conf.json`).
3. **Reads go through the app's services.** `deino-core::query` (`Library`)
   composes the existing repositories and services: `ExportAssembler` for
   attendee and speaker names and the transcript, `NoteRenderer` and
   `EnhancedNotesReader` for the notes (the newest enhanced-notes version,
   with its citations as `deino://meeting/<id>?t=<ms>` links),
   `ActionItemService` for action items, `FolderService` for the tree, and
   ORG-08/AI-01's `Retriever` for search behind a narrow `Search` port
   (`HybridSearch`). The read layer adds no SQL of its own.
5. **Semantic search when the app's model is there.** When the app has
   downloaded `nomic-embed-text-v1.5` (`models/embedding/` beside the
   database) and `llama-helper` sits beside the binary, the CLI's
   `HelperEmbedder` runs that sidecar: one process per `deino` process,
   owned by a worker thread that always reads each reply (so a search past
   its 300 ms deadline cannot desynchronise it), restarted after the helper's
   idle exit, killed and reaped when the embedder drops, and ending on its
   own at the end of stdin if `deino` is killed. The model loads in
   `Embedder::ready`, outside the query deadline. Otherwise search is
   keyword-only (FTS5). Every search result says which `mode` ran.
4. **Bundled as a sidecar, installed as a symlink.** The app bundles the
   binary as the `cli` sidecar (`Deino.app/Contents/MacOS/cli`). Settings →
   Developer links it as `deino` into `~/.local/bin` or `/usr/local/bin`
   after a confirmation, never replacing a file it did not make, and shows the
   `claude mcp add deino -- <path> mcp` command and the Claude Desktop config.
   The sidecar is called `cli` (as Zed's is) because Tauri copies sidecars
   into `target/<profile>/` under their own name and refuses the package
   name `deino`: a sidecar named after the crate's binary (`deino-cli`) would
   overwrite the freshly built one there.

## Alternatives

- **HTTP MCP inside the app with a stdio shim (the original DEV-02).** Needs
  the app running, a port, a token and a second binary anyway; read access
  through the database needs none of it. Write tools will need the app (one
  writer, `DbChanged` for the UI); they come later through it.
- **A CLI over the local REST API (the original DEV-04).** Waits for DEV-03
  and needs the app running; same answer.
- **Raw SQL in the CLI.** Duplicates what names a speaker, which notes are
  current and which lines are deleted; the app's services already decide.
- **`immutable=1` instead of `mode=ro`.** Skips locking, so a read during the
  app's checkpoint could see a torn page. `mode=ro` follows the WAL safely.
- **Bundling as a Tauri resource.** Executables belong in `Contents/MacOS`
  for signing and notarization.

## Consequences

- Claude Code reads meetings with or without the app, and the database is
  never written: tests compare the file's SHA-256 and modification time
  before and after every command and check an older schema is refused and
  left unmigrated. A WAL reader may create SQLite's empty `-wal`/`-shm` side
  files beside the database; the main file never changes.
- A CLI and an app from different builds refuse each other's schema with an
  instruction instead of misreading; reinstalling from Settings fixes it.
- Write tools, `ask_deino` and an HTTP transport are follow-ups (PARITY_PLAN
  §7.2).
- Times print in the Mac's zone; `TZ` overrides it (the tests use `UTC`).
- The build stages one more sidecar: `scripts/stage-cli.sh` (release:
  `--release --target <triple>` in `release-macos.yml`); `ci-local.sh`
  stages a self-describing placeholder like `llama-helper`'s.
