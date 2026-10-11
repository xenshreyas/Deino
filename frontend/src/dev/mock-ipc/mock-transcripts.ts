/**
 * The mock `transcript_segments` and `speakers` tables: every fixture
 * meeting's transcript as the Rust seeder writes it (`fixtureUuid` IDs,
 * speakers named after the people they are), the lines the mock recorder
 * saves, and, on request, one very long generated meeting for performance
 * checks (`long-meeting.ts`).
 */
import type { MeetingId } from '@/features/meetings';
import type * as Generated from '@/lib/ipc/generated/transcript';

import { fixtureUuid } from './fixture-ids';
import type { FixtureMeeting } from './fixture-schema';

/** The fixture user's address (`deino_testkit::fixture::USER_EMAIL`). */
const USER_EMAIL = 'shreyas@deino.dev';

/** `PaletteColor::cycle`: slots 1–6 by speaker index. */
function paletteSlot(index: number): Generated.PaletteColor {
  return (index % 6) + 1;
}

/** The fixture's speakers, as `transcript|get` names them. */
function fixtureSpeakers(fixture: FixtureMeeting): Generated.TranscriptSpeaker[] {
  return fixture.speakers.map((speaker, index) => {
    const person = fixture.attendees.find(({ email }) => email === speaker.attendee);
    return {
      id: fixtureUuid(`speaker/${fixture.id}/${speaker.label}`) as Generated.SpeakerId,
      name: person?.name ?? speaker.label,
      personId:
        person === undefined ? null : (fixtureUuid(`person/${person.email}`) as Generated.PersonId),
      isMe: speaker.attendee === USER_EMAIL,
      color: paletteSlot(index),
    };
  });
}

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

/** The transcripts of the mock database. */
export class MockTranscripts {
  private readonly transcripts = new Map<string, Generated.Transcript>();

  constructor(fixtures: readonly FixtureMeeting[]) {
    for (const fixture of fixtures) {
      this.transcripts.set(fixture.id, {
        segments: fixtureSegments(fixture),
        speakers: fixtureSpeakers(fixture),
      });
    }
  }

  /** `transcript|get`: a copy, empty for a meeting without one. */
  get(meetingId: MeetingId): Generated.Transcript {
    const transcript = this.transcripts.get(meetingId);
    if (transcript === undefined) return { segments: [], speakers: [] };
    return { segments: [...transcript.segments], speakers: [...transcript.speakers] };
  }

  /** Stores a whole transcript (a generated meeting). */
  put(meetingId: MeetingId, transcript: Generated.Transcript): void {
    this.transcripts.set(meetingId, transcript);
  }

  /** A new recording's Me and Them speakers, as `RecordingService::start` creates them. */
  startRecording(meetingId: MeetingId): void {
    const speaker = (name: string, isMe: boolean): Generated.TranscriptSpeaker => ({
      id: crypto.randomUUID() as Generated.SpeakerId,
      name,
      personId: null,
      isMe,
      color: isMe ? 1 : 2,
    });
    this.put(meetingId, { segments: [], speakers: [speaker('Me', true), speaker('Them', false)] });
  }

  /** Saves a finished live line on its channel's speaker. */
  save(segment: Omit<Generated.Segment, 'speakerId'>): void {
    const transcript = this.transcripts.get(segment.meetingId) ?? { segments: [], speakers: [] };
    const isMe = segment.channel === 'mic';
    const speaker = transcript.speakers.find((candidate) => candidate.isMe === isMe);
    const saved = { ...segment, speakerId: speaker?.id ?? null };
    this.put(segment.meetingId, { ...transcript, segments: [...transcript.segments, saved] });
  }
}
