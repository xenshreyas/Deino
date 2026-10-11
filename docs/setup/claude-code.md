# Claude Code (and Claude Desktop): read your meetings

Two minutes. Claude gets read-only access to your meetings, notes, AI notes,
transcripts, action items, people, tags, folders and calendar through the
`deino` command's MCP server. It reads the database on your Mac directly (the
app does not need to be running) and can never change anything (ADR 0035).
Your meetings go only to the Claude client you connect.

## 1. Install the command (30 seconds)

Deino → **Settings → Developer → Command-line tool** → **Install** next to
`~/.local/bin`, then confirm. This creates `~/.local/bin/deino`, a link to
the tool inside the app (`Deino.app/Contents/MacOS/cli`).

Make sure `~/.local/bin` is on your `PATH` (it is by default with many
setups); otherwise add this to `~/.zshrc` and open a new Terminal:

```sh
export PATH="$HOME/.local/bin:$PATH"
```

Check it:

```sh
deino status          # which database it reads, and the latest meeting
deino meetings -n 3   # your three most recent meetings
```

`/usr/local/bin` works too; when the app cannot write there, Settings shows
the `sudo ln -sf …` command to run instead.

## 2. Connect Claude Code (30 seconds)

Copy the command from Settings → Developer → Claude, or run:

```sh
claude mcp add --scope user deino -- ~/.local/bin/deino mcp
```

`--scope user` makes it available in every project; leave it out to add it
to the current project only. Check it with `claude mcp list` (deino should
be ✔ connected), then ask Claude Code things like:

- "List my last 3 meetings and their action items."
- "What did I commit to last week?"
- "What did Maya say about the release cut? Quote her."
- "Summarise every meeting with Brightkite."

## 3. Connect Claude Desktop (optional)

Settings → Developer → Claude → **Claude Desktop** has the snippet. Merge it
into `~/Library/Application Support/Claude/claude_desktop_config.json` and
restart Claude Desktop:

```json
{
  "mcpServers": {
    "deino": { "command": "/Users/<you>/.local/bin/deino", "args": ["mcp"] }
  }
}
```

Desktop apps do not read your shell's `PATH`, so the snippet uses the full
path.

## What Claude can call

| Tool | Command | What it reads |
|---|---|---|
| `list_meetings` | `deino meetings` | Meetings, newest first; filter by `--from/--to` (local days), `--tag`, `--folder`, `--person`, `--company`, `-n` |
| `read_meeting` | `deino meetings read <ids…>` | Title, date, length, attendees, tags, folders, your notes, the AI notes (with links to cited moments), action items, recording paths |
| `get_transcript` | `deino transcripts read <id>` | Speaker-attributed, timestamped lines, a window at a time (`--from-ms`, `--to-ms`, `-n`) |
| `search` | `deino search <words…>` | Deino's own search over titles, notes, AI notes, action items, transcript moments, people, companies and folders: keywords, plus meaning when the app's embedding model is downloaded (the result says `keyword` or `hybrid`) |
| `list_action_items` | `deino action-items` | `--assignee me\|others\|anyone\|unassigned\|<name>`, `--status open\|done\|any`, `--due overdue\|today\|this_week\|no_date` |
| `list_people`, `list_companies`, `list_tags`, `list_folders` | `deino people`, `companies`, `tags`, `folders` | Names Claude can filter meetings by |
| `list_calendar_events` | `deino calendar --from --to` | Synced calendar events |
| | `deino export <id> -f md\|txt\|json\|pdf\|docx\|srt\|vtt` | A meeting as the app's Share menu exports it |

Every command takes `--json`. Links look like `deino://meeting/<id>?t=<ms>`
(the moment in the recording). Names (tags, folders, people, companies) can
be IDs, names, emails or domains; an ambiguous name lists the matches.

## Troubleshooting

- **"no Deino database at …"**: open the Deino app once, or point the tool at
  another file with `--db <path>` or `DEINO_DB=<path>`.
- **"the Deino database is newer than this binary understands"**: the app was
  updated; click **Reinstall** in Settings → Developer.
- **"… older than this binary expects"**: open the updated app once; it
  upgrades the database.
- **Claude Code shows deino as failed**: run `deino status` in Terminal for
  the reason, then `claude mcp remove deino` and add it again.

## Try it on sample data

Developers can try everything without real meetings:

```sh
cargo run -p deino-cli --example sample_db -- /tmp/deino-sample.sqlite
cargo run -p deino-cli -- --db /tmp/deino-sample.sqlite meetings
claude mcp add deino-sample -- "$PWD/target/debug/deino-cli" --db /tmp/deino-sample.sqlite mcp
```

A dev build of the app (`pnpm tauri:dev`) finds the tool after
`scripts/stage-cli.sh`, which builds it and stages it as the `cli` sidecar.
