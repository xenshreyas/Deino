/**
 * The mock `transcript_segments` table: every fixture meeting's transcript
 * as the Rust seeder writes it (`fixtureUuid` segment and speaker IDs, the
 * same ones `mock-speakers.ts` names), the lines the mock recorder saves,
 * and, on request, one very long generated meeting for performance checks
 * (`long-meeting.ts`).
 */
import type { MeetingId } from '@/features/meetings';
import type * as Generated from '@/lib/ipc/generated/transcript';

import { fixtureUuid } from './fixture-ids';
import type { FixtureMeeting } from './fixture-schema';

/** The fixture's segments, in spoken order. */
function fixtureSegments(fixture: FixtureMeeting): Generated.Segment[] {
  return fixture.segments.map((segment, index) => ({
    id: fixtureUuid(`segment/${fixture.id}/${String(index)}`) as Generated.SegmentId,
    meetingId: fixture.id as Generated.MeetingId,
    channel: segment.channel,
    speakerId: fixtureUuid(`speaker/${fixture.id}/${segment.speaker}`) as Generated.SpeakerId,
    start: segment.startMs,
    end: segment.endMs,
    text: segment.text,
    words: segment.words.map((word) => ({ text: word.text, start: word.startMs, end: word.endMs })),
    isDeleted: false,
    editedAt: null,
  }));
}

/** Who said a segment now, after renames (`mock-speakers.ts`); `undefined` if unknown. */
export type SpeakerOf = (segment: Generated.SegmentId) => Generated.SpeakerId | undefined;

/** The transcripts of the mock database. */
export class MockTranscripts {
  private readonly segments = new Map<string, Generated.Segment[]>();

  constructor(fixtures: readonly FixtureMeeting[]) {
    for (const fixture of fixtures) this.segments.set(fixture.id, fixtureSegments(fixture));
  }

  /** `transcript|get`, each segment on its current speaker (`speakerOf`). */
  get(meetingId: MeetingId, speakerOf: SpeakerOf = () => undefined): Generated.Transcript {
    const segments = (this.segments.get(meetingId) ?? []).map((segment) => ({
      ...segment,
      speakerId: speakerOf(segment.id) ?? segment.speakerId,
    }));
    return { segments };
  }

  /** Stores a whole transcript (a generated meeting). */
  put(meetingId: MeetingId, transcript: Generated.Transcript): void {
    this.segments.set(meetingId, [...transcript.segments]);
  }

  /** Saves a finished live line; the mock recorder stores no speakers. */
  save(segment: Generated.Segment): void {
    const saved = this.segments.get(segment.meetingId) ?? [];
    this.segments.set(segment.meetingId, [...saved, segment]);
  }
}
