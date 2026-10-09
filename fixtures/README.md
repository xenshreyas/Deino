# Fixtures

Eight realistic, fictional English meetings: the test bed for the meeting UI
(`pnpm dev:mock`), repository tests (`deino_testkit::FixtureSeeder`) and every
AI feature's evals (notes, action items, insights, chat).

| File | Kind | Recorded | Length | Channels | Notes | Audio |
|---|---|---|---|---|---|---|
| `planning.json` | Q4 planning, 8 people | Mon 28 Sep 2026 | 83 min | mic + system | yes | |
| `board-update.json` | board update, hybrid room | Tue 29 Sep | 37 min | 2 mic + system | yes | |
| `sales-discovery.json` | external prospect, uninvited speaker | Wed 30 Sep | 26 min | mic + system | yes | |
| `brainstorm.json` | in person, one shared mic | Thu 1 Oct | 29 min | mic only | yes | |
| `interview.json` | system-design interview | Fri 2 Oct | 31 min | mic + system | yes | |
| `customer-support.json` | customer escalation | Mon 5 Oct | 17 min | mic + system | no | |
| `one-on-one.json` | weekly 1:1 | Wed 7 Oct | 19 min | mic + system | yes | first 54 s |
| `standup.json` | daily standup, 5 people | Thu 8 Oct | 7 min | mic + system | no | first 53 s |

The user is always **Shreyas** (`shreyas@deino.dev`), CTO of Deino, a
fictional field-service scheduling startup. Every person, company, domain and
number is invented; the meetings share one world (the same Q3 figures, the
Calendar Sync v1.4 incident, the Q4 priorities), so cross-meeting questions
have real answers.

## Schema

One JSON object per file, camelCase, validated twice: by serde in
`crates/deino-testkit/src/fixture/` (unknown fields fail, then cross-field
rules in `validate.rs`) and by zod in `frontend/src/dev/mock-ipc/fixture-schema.ts`.

- `id` (`meeting-fixture-<file name>`), `title`, `kind`, `startedAt`, `durationMs`
- `calendarEvent`: provider and iCal IDs, title, invite body, times,
  `platform` (`google_meet`, `zoom`, `teams`, `in_person`), `conferenceUrl`
  or `location`, `organizerEmail`, `recurrence`
- `companies`: `{ name, domain }` for every company an attendee belongs to
- `attendees`: `{ name, email, title, role, rsvp }`; some declined and never speak
- `speakers`: `{ label: "Speaker N", attendee: email | null, channel }`, in
  order of first appearance; `null` is someone who was not invited
- `segments`: `{ speaker, channel, startMs, endMs, text, words[] }`, where
  each word is `{ text, startMs, endMs }`
- `userNotes`: the rough markdown the user typed during the meeting, or `null`
- `groundTruth`: `decisions[]`, `actionItems[]` (`title`, `assignee` email or
  `null`, `due` date or `null`, `evidence`) and `facts[]` (`question`,
  `answer`, `evidence`). Every `evidence` is an exact quote of one segment.
- `audio`: `{ mic, system, clipEndMs }` paths under `fixtures/`, or `null`

## Changing a fixture

The JSON is the source of truth. Edit `text` (or add segments with only
`speaker` and `text`, plus an optional `pauseMs` silence before them), then
recompute the timing:

```bash
node fixtures/tools/retime.mjs fixtures/meetings/<file>.json
node fixtures/tools/synthesize-audio.mjs fixtures/meetings/<file>.json   # audio fixtures only (macOS)
cargo test -p deino-testkit --features test-support                    # validates every rule
```

`retime.mjs` derives channels, segment and word timings and the duration from
the text with a speaking-rate model (about 150 words a minute), keeping every
existing silence. `synthesize-audio.mjs` speaks the first ~55 s with macOS
`say` (one voice per person), fitting each segment into its own slot on its
own track, and encodes 16 kHz mono AAC with `afconvert`.
