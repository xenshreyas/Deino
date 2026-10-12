# 0036. One entity service: merges as aliases, aggregates computed on read

- **Status:** Accepted
- **Date:** 2026-10-11
- **Card:** ORG-04 (People and Companies)

## Context

People and companies arrive from several places: calendar invitations
(ORG-01's `AttendeeDirectory`), speakers the user names (CAP-04's rename),
and @mentions in the notepad. Before ORG-04 each source had its own insert
code, with slightly different keys (ORG-01 by email, CAP-04 always a new
row). The People and Companies pages need four more things:

1. **Merging** duplicates, moving every link: meeting attendance, transcript
   speakers, voiceprints and voice suggestions (CAP-04), action-item
   assignments (AI-05), @mentions in the user's notes and in the enhanced
   notes (NOT-02), people of a company.
2. **Lists sorted by last met**, with meeting counts. PARITY_PLAN §3A names
   `people.last_met_at` and `people.meeting_count` as candidate cached
   aggregates.
3. **AI briefings** per person and company, refreshed after each meeting.
4. **Company logos** from `https://<domain>/favicon.ico`, fetched once, under
   the network egress rules (FND-12, ADR 0019).

## Decision

**One rule set.** `deino_core::entities::EntityDirectory` owns the upsert
rules and every source calls it: people are keyed by email, else by their
exact name (ignoring case), so naming the same speaker twice never makes two
people; companies are keyed by the email's domain and never come from a
public mail provider or a personal vanity domain (`jordanellis.dev` for
Jordan Ellis). Upserts only add. ORG-01's `AttendeeDirectory` keeps its
filters (declined events, broadcasts) and delegates; CAP-04's "type a new
name" goes through it too.

**Merges keep the absorbed row as an alias.** A merged person (or company)
stays as a hidden row with `merged_into` pointing at the survivor, so its
address or domain still resolves when a calendar names it again (the
directory follows `merged_into`). The survivor fills its gaps from the
absorbed row (address, title, company, avatar, domain, logo, summary) and
keeps both sets of notes. `EntityMerger` applies a merge in one transaction:
it writes the absorbed row first (freeing a unique address or the "Me" flag),
then the survivor, then moves every link and flattens earlier merges.
Mentions live inside opaque TipTap JSON; they move by replacing the UUID
text, which is exact for a 36-character UUID. Voiceprints keep the
survivor's best five, the cap CAP-04's enrolment keeps.

**Aggregates are computed on read, not stored.** `person_rows.sql` and
`company_rows.sql` count each person's (and company's) meetings outside the
trash that already happened, through `idx_meeting_attendees_person_id` and
the meetings primary key. Measured with `list_bench` (release build, 2,000
people at 200 companies, 5,000 meetings, 20,000 attendance rows, machine
load average above 200 from parallel agents): the People list in 39 ms and
the Companies list in 19 ms.

**Briefings are a pipeline stage.** `update_entities` (order 90) links named
speakers who were not invited as `speaker_only` attendees, then refreshes
the briefing of every attendee except the user and of their companies (at
most twelve per meeting) on the `entity.summary` route, reading each one's
latest eight meetings' enhanced notes and the user's notes, plus their open
action items. Without a gateway it links speakers and skips briefings
(`SummaryOutcome::NoModel`). Summaries are written by single-field updates,
so a briefing never overwrites notes the user is typing.

**Logos go through a new `favicons` service.** `HostPattern::AnyPublicName`
lets it reach any public DNS name (never an IP address, this Mac or a
`.local`-style name). Like every integration it is **off until the user
switches it on** (Settings → Network, "Company logos"), and offline mode
stops it. A fetch the policy refuses is not recorded, so logos appear once
allowed; any other outcome is recorded in `companies.logo_checked_at` and the
site is not asked again unless the user picks "Fetch logo again". Only
ICO, PNG, GIF, JPEG and WebP up to 512 KB are kept; SVG is refused.

## Alternatives

- **Denormalised `last_met_at` and `meeting_count` with triggers.** Rejected:
  attendance has many writers (calendar linking, speaker naming, manual
  edits, merges, ORG-07's meeting merge) and the meeting side changes too
  (trash, restore, status, start time). Triggers on both tables would keep
  it correct but would also turn every attendance write into a `people`
  write, and the measured read cost is already well under a frame. Revisit
  if the lists pass about 20,000 people or the bench exceeds 100 ms.
- **Deleting the absorbed row on merge.** Rejected: its address or domain
  would be free again, and the next calendar sync would recreate the
  duplicate.
- **A `company_domains` alias table.** Equivalent for companies, but a second
  shape next to `people.merged_into`; one pattern for both is simpler.
- **Parsing and rewriting mention JSON with `deino-doc`.** Correct but slower
  and needs every document loaded in Rust; the UUID text replacement is exact
  and runs in SQL inside the merge transaction.
- **Favicons on by default.** Rejected by FND-12: a service sends nothing
  until the user turns it on, and a company's own site learning that the
  user met someone there is exactly the kind of egress the user should
  choose.

## Consequences

- One place to change who becomes a person or company; CAP-04, ORG-01 and
  the notepad agree by construction.
- A merge must learn about every new table that references `people`:
  `merge_repo.rs` lists them, and a card that adds such a table adds one
  statement there (and a test in `person_service/merge_tests.rs`).
- Merged rows are invisible to lists but visible to `find`; readers resolve
  them with `EntityDirectory::surviving_person`/`surviving_company`.
- Logos are absent until the user allows company logos; the page says where.
