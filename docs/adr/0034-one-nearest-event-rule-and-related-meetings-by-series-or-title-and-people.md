# 0034. Link events with one nearest-event rule; relate meetings by series, or by title and people

- **Status:** Accepted
- **Date:** 2026-10-11
- **Card:** ORG-03 (`docs/research/granola.md` §3.1, §6.5;
  `docs/research/circleback.md` §3.5)

## Context

ORG-03 links a meeting to its calendar event when recording starts, from a
picker in the meeting header (also mid-recording), and shows related
meetings with "Previous in series". Three things already touched this:

- CAP-06's detection named its prompt after an event with its own window
  (started under 15 minutes ago or starting within 5,
  `deino_detect::calendar::pick_linked`), but nothing stored a link: a
  recording started from a detection only took the event's title.
- Recording is started from three places (the record screen, the tray, a
  detected call), all through `deino_recording::RecordingService::start`.
- NOT-02's regenerate banner compares a stored `EnhanceBasis` with the
  meeting now; the basis knew attendees but not the linked event.

Granola relates notes by the recurring event or an exact title match. An
exact title alone relates every team's "Weekly sync".

## Decision

- **One rule, in the core.** `deino_core::calendar::linking::nearest_event`:
  a timed event the user did not decline, under way or starting within
  15 minutes, the start closest to now first (a tie goes to the one under
  way). Back to back, the next meeting wins once its start is closer.
  Detection's `SyncedCalendar` uses it too; `deino-detect` keeps only the
  `CalendarLookup` port.
- **Auto-link through a start observer.** `deino_recording::StartObserver`
  hears every start once the meeting row exists and may return a new title
  (the recorder shows it). `RecordingService::observing_starts` sets it, so
  `RecordingDeps` is unchanged. The app's `LinkOnStart` calls
  `EventLinkService::auto_link`; a start the caller titled keeps its title.
- **Linking is three idempotent writes:** the link and series
  (`MeetingLifecycleRepo::link`), the attendees (`EventAttendance`: invitees
  who did not decline, found or added through
  `AttendeeDirectory::person_for`, the user as "Me", no new people from
  broadcasts; re-linking removes the previous event's invitees unless they
  only spoke), then the title. Repeating a link converges, so the writes
  are not one transaction across repositories.
- **The picker suggests the day before the meeting** (its recording start,
  else its creation) up to an hour after, closest first, from the visible
  calendar (`EventAgenda`, implemented by `CalendarService::upcoming`).
- **Related meetings:** the same `series_id`, or exactly the same title and
  the same attendees besides the user (a non-empty set). The latest one
  before the meeting is "Previous in series". The lookup is a union of the
  series index and a new partial title index, never a scan.
- **A typed regenerate reason.** `EnhanceBasis` records the linked event as
  `EventBasis` (`unknown` for versions written before ORG-03, which never
  yields a reason); a different event, or none, is
  `RegenerateReason::EventChanged`.

## Alternatives

- **Keep detection's window and add a second one for recordings.** Two
  rules would link a detected call and its recording to different events.
- **Link in each start path (command, tray, detection).** Three call sites
  to keep in step, and a fourth start path would forget.
- **A required `RecordingDeps` field for the observer.** Every rig and
  contract test building the deps would change for a hook most of them do
  not need.
- **Exact title alone for related meetings (Granola).** Relates unrelated
  teams' meetings that share a generic title.
- **One transaction for the link, attendees and title.** Needs a
  cross-repository unit of work for writes that are already idempotent.

## Consequences

- A detected call and its recording link the same event; detection now also
  links an event starting 6–15 minutes ahead, which it used not to.
- The recorder's live title follows the event at start; a mid-recording
  re-link renames the meeting row, but the sidebar's live row keeps the
  start title until it reads the row (follow-up for the record panel).
- Attendees from a link are ordinary `meeting_attendees` rows, so ORG-04's
  chips, people pages and the enhanced notes see them with no extra wiring.
- Revisit the window if users record well before or after events, and the
  people rule if recurring meetings change guests often.
