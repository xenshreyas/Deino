/**
 * Mock of `@/lib/ipc/generated/enhance`: the enhanced notes over the mock
 * `enhanced_notes` table. Writing notes takes a moment (so the "writing"
 * state shows) and is refused with `conflict` over the user's edits, like
 * Rust; citations are answered from the fixtures' transcripts.
 */
import type * as Generated from '@/lib/ipc/generated/enhance';

import { mockDatabase, mockEnhanced, mockEventLinks, mockNotes } from './database';
import { fixtureUuid } from './fixture-ids';
import type { FixtureMeeting } from './fixture-schema';
import { latency } from './latency';
import { failure, ok } from './mock-database';
import { transcriptEditedAt } from './mock-transcript-edits';

/** How long the mock "writes" notes. */
const WRITING_MS = 1600;

function notFound(what: string) {
  return failure<never>({ code: 'not_found', message: `${what} was not found` });
}

/** The excerpts of `ids` in the fixture, in time order. */
function cited(fixture: FixtureMeeting, ids: readonly string[]): Generated.CitedSegment[] {
  return fixture.segments.flatMap((segment, index) => {
    const segmentId = fixtureUuid(`segment/${fixture.id}/${String(index)}`) as Generated.SegmentId;
    if (!ids.includes(segmentId)) return [];
    const speaker = fixture.speakers.find((candidate) => candidate.label === segment.speaker);
    const person = fixture.attendees.find((attendee) => attendee.email === speaker?.attendee);
    return [
      {
        segmentId,
        start: segment.startMs,
        end: segment.endMs,
        speaker: person?.name ?? segment.speaker,
        text: segment.text,
      },
    ];
  });
}

function view(meetingId: Generated.MeetingId, version: number | null) {
  const notesSavedAt = mockNotes.get(meetingId).updatedAt;
  const shown = mockEnhanced.view(
    meetingId,
    version,
    notesSavedAt,
    mockEventLinks.change(meetingId),
  );
  return withTranscriptEdits(shown, transcriptEditedAt(meetingId));
}

/** Adds "the transcript was edited" when it was, after the version was written (TRN-02). */
function withTranscriptEdits<T extends Generated.EnhancedNotesView | null | undefined>(
  shown: T,
  editedAt: string | null,
): T {
  if (!shown?.isLatest || editedAt === null || editedAt <= shown.version.createdAt) return shown;
  // Rust's order: the calendar's reasons, then the transcript, then the notes.
  const isCalendar = (reason: Generated.RegenerateReason) =>
    reason === 'eventChanged' || reason === 'attendeesChanged';
  const calendar = shown.reasons.filter(isCalendar);
  const rest = shown.reasons.filter((reason) => !isCalendar(reason));
  return { ...shown, reasons: [...calendar, 'transcriptEdited', ...rest] };
}

export const commands = {
  async get(meetingId, version) {
    await latency();
    if (mockDatabase.meeting(meetingId).status === 'error') return notFound(`meeting ${meetingId}`);
    const shown = view(meetingId, version);
    return shown === undefined ? notFound(`version ${String(version)} of the notes`) : ok(shown);
  },
  async versions(meetingId) {
    await latency();
    return ok(mockEnhanced.versions(meetingId));
  },
  async enhance(meetingId, replaceEdits) {
    // Like Rust: refuse to replace edits before writing anything.
    if (mockEnhanced.isEdited(meetingId) && !replaceEdits) {
      await latency();
      return failure({
        code: 'conflict',
        message: 'Regenerating will replace the notes you edited.',
      });
    }
    await new Promise((resolve) => setTimeout(resolve, WRITING_MS));
    const version = mockEnhanced.write(meetingId, new Date());
    mockDatabase.touch(['enhanced_notes']);
    const written = view(meetingId, version);
    return written ? ok(written) : notFound(`meeting ${meetingId}`);
  },
  async save(meetingId, version, doc) {
    await latency();
    const now = new Date();
    if (!mockEnhanced.save(meetingId, version, doc, now)) {
      return failure({
        code: 'conflict',
        message: 'Only the newest version of the notes can be edited.',
      });
    }
    mockDatabase.touch(['enhanced_notes']);
    return ok({ meetingId, version, updatedAt: now.toISOString() });
  },
  async restore(meetingId, version) {
    await latency();
    const restored = mockEnhanced.restoreVersion(meetingId, version, new Date());
    if (restored === null) return notFound(`version ${String(version)} of the notes`);
    mockDatabase.touch(['enhanced_notes']);
    const shown = view(meetingId, restored);
    return shown ? ok(shown) : notFound(`meeting ${meetingId}`);
  },
  async cited(meetingId, segmentIds) {
    await latency();
    const fixture = mockDatabase.fixture(meetingId);
    return ok(fixture === undefined ? [] : cited(fixture, segmentIds));
  },
} satisfies typeof Generated.commands;
