# deino-cli

## Role

The `deino` command (DEV-02, DEV-04, ADR 0033): the user's meetings, notes,
transcripts and action items read straight from the Deino database, in the
terminal (`deino meetings`, `--json` everywhere) and over MCP for Claude
(`deino mcp`, stdio). Setup for people: `docs/setup/claude-code.md`.

## Owns

- `cli.rs`: the `clap` command line and `execute`; `main.rs` prints and maps
  failures to exit codes (3 not found, 4 invalid, 5 database unavailable).
- `request.rs`, `request/`: each read once, as a `Request` (clap flags, serde
  arguments, schemars input schema, a typed output object): `list_meetings`,
  `read_meeting`, `get_transcript`, `search`, `list_action_items`,
  `list_people`, `list_companies`, `list_tags`, `list_folders`,
  `list_calendar_events`.
- `markdown.rs`, `markdown/`: the compact Markdown both faces print.
- `mcp.rs`, `mcp/registry.rs`: the MCP server (`rmcp`), one tool per
  `Request`, tool errors as `{"error": {"code", "message"}}`.
- `db_path.rs`: `--db`, `DEINO_DB`, else the app's own file.
- `embedder.rs`, `embedder/worker.rs`: `HelperEmbedder`, semantic search
  through the bundled `llama-helper` and the app's downloaded
  `nomic-embed-text-v1.5` (only when both are there; `DEINO_LLAMA_HELPER`
  overrides the helper), one helper per process on a worker thread, stopped
  and reaped on drop.
- `export.rs`: `deino export` through `deino-core::export` and `deino-export`.
- `examples/sample_db.rs`: a sample database for trying it out.

## Does Not Own

- What a meeting is made of and how it reads: `deino-core::query` (over the
  app's repositories and services).
- Opening the file: `deino_db::Database::open_read_only`.
- Installing the binary and the Claude snippets: `deino-core::cli_setup`
  and the app's `developer` plugin.

## Invariants

- Never writes, creates or migrates the database; refuses a schema it was
  not built for (`tests/read_only.rs` hashes the file around every command).
- `deino mcp` writes nothing to stdout but the protocol, and logs nothing.
- Every MCP tool is a `Request`, so the CLI and the tools cannot drift; a new
  tool is a `Request` and one line in `mcp::registry::tools`.
- Tool failures are tool results (`isError`), never protocol errors, so the
  model can read them; only an unknown tool name is a protocol error.
- No meeting content in errors: IDs, names the caller typed and paths only.
- The Cargo binary is `deino-cli` (the app's executable is `deino`); the app
  bundles it as the `cli` sidecar and installs it as `deino`.
