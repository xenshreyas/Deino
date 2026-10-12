# Shell scripts after meetings

An automation step can run a command of yours after a meeting, with the
meeting as JSON on stdin (AUT-01, ADR 0042). Use it to append to a log, file
notes into a vault, or call a CLI.

## Turn it on

Scripts are **off by default**. On the Automations page, turn on **Allow
shell scripts** (the setting `automations.shellScripts`). Deino checks it
every time a script is about to run, so turning it off stops every script at
once; existing script steps stay and show a warning.

## How a script runs

- `/bin/sh -c "<your command>"`, as you, with your permissions.
- Working directory: the meeting's folder, `<app data>/recordings/<meeting id>/`.
- stdin: the payload below, then end of file.
- Time limit: 60 seconds. After that the script and every process it started
  (its process group) are killed and the step fails.
- Exit status 0 is success. The last 400 characters of stdout show in the
  run log as the step's output; on failure, the end of stderr is in the
  error. Deino never logs either.
- When **Hide sensitive information** is on (Settings → Network), the
  payload is redacted like every export.

## The payload (`deino.automation` version 1)

Fields you did not tick under "Payload includes" are left out.

```json
{
  "format": "deino.automation",
  "version": 1,
  "automation": { "id": "0192…", "name": "Discovery recap" },
  "trigger": { "kind": "pipeline" },
  "meeting": {
    "id": "meeting-…",
    "title": "Deino × Brightkite Logistics: discovery",
    "startedAt": "2026-10-08T14:00:00+02:00",
    "duration": 2712,
    "tags": ["Sales"],
    "folders": ["Customers"]
  },
  "link": "deino://meetings/meeting-…",
  "notes": "## Summary\n…",
  "actionItems": [
    { "title": "Send pricing", "description": "", "assignee": "Me", "due": "2026-10-10", "done": false }
  ],
  "transcript": [
    { "speaker": "Marcus Webb", "isMe": false, "start": 4310, "end": 9020, "text": "…" }
  ],
  "insights": [{ "name": "Discovery scorecard", "answers": [{ "value": { "kind": "text", "text": "…" }, "speaker": "Marcus Webb" }] }],
  "attendees": [{ "name": "Marcus Webb", "email": "marcus@brightkite.example", "isMe": false }],
  "audioPath": "/Users/you/Library/Application Support/…/recordings/meeting-…/audio.m4a",
  "steps": [{ "kind": "insight", "insightId": "0192…", "name": "Discovery scorecard", "answers": [] }]
}
```

- `trigger.kind` is `pipeline` (after processing), `tag_applied` (with
  `tagId`) or `manual`.
- `steps` holds what earlier steps of the same run produced, so a script
  after a "Generate insight" step receives its answers.
- A new field may appear in version 1; a field that changes meaning or goes
  away bumps `version`.

## Examples

```sh
# Append each meeting's title to a log
jq -r .meeting.title >> ~/meetings.log

# Keep the whole payload next to the recording
cat > payload.json
```
