/**
 * Speakers for `pnpm dev:mock`: an in-memory twin of `deino_core::speakers`
 * over the fixture transcripts (the seeder's speaker and segment IDs), with
 * its rules: rename one segment or all of them (which saves a voice
 * sample), merge, confirm or dismiss a suggestion. Writes report their
 * tables as `DbChanged`.
 */
import type * as Generated from '@/lib/ipc/generated/speakers';
import type { AppError, Table } from '@/lib/ipc/generated/shared';
import type { CommandResult } from '@/lib/ipc/result';

import { failure, ok } from './command-result';
import { fixtureUuid } from './fixture-ids';
import type { FixtureMeeting } from './fixture-schema';
import { MockVoiceprints } from './mock-voiceprints';

type MeetingId = Generated.MeetingId;
type PersonId = Generated.PersonId;
type SpeakerId = Generated.SpeakerId;

/** A person the mock knows (the fixtures' people). */
export interface MockPerson {
  id: PersonId;
  name: string;
  email: string | null;
  isMe: boolean;
}

/** One stored speaker. */
interface Row {
  speakerId: SpeakerId;
  meetingId: MeetingId;
  label: string;
  color: number;
  isMe: boolean;
  personId: PersonId | null;
  suggestion: { personId: PersonId; score: number } | null;
}

/** One segment's owner and length. */
interface Said {
  meetingId: MeetingId;
  speakerId: SpeakerId;
  length: number;
}

const notFound = (what: string): AppError => ({
  code: 'not_found',
  message: `${what} was not found`,
});

/** The speakers of every fixture meeting, and naming them. */
export class MockSpeakers {
  private rows: Row[] = [];
  private said = new Map<Generated.SegmentId, Said>();
  readonly voiceprints: MockVoiceprints;

  constructor(
    fixtures: readonly FixtureMeeting[],
    private readonly people: MockPerson[],
    private readonly onChange: (tables: Table[]) => void,
  ) {
    this.voiceprints = new MockVoiceprints(people, onChange);
    for (const fixture of fixtures) this.load(fixture);
  }

  /** The meeting's participants, most speech first. */
  participants(meetingId: MeetingId): Generated.Participant[] {
    const rows = this.rows.filter((row) => row.meetingId === meetingId);
    const speech = (row: Row) => this.speechOf(row.speakerId);
    const total = rows.reduce((sum, row) => sum + speech(row), 0);
    return rows
      .map((row) => this.participant(row, speech(row), total))
      .sort((a, b) => b.speech - a.speech || a.label.localeCompare(b.label));
  }

  /** Attendees of the meeting first, then everyone matching `query`. */
  candidates(meetingId: MeetingId, query: string): Generated.NameCandidate[] {
    const prefix = query.trim().toLowerCase();
    const matches = (person: MockPerson) =>
      prefix === '' ||
      person.name.toLowerCase().startsWith(prefix) ||
      person.name
        .toLowerCase()
        .split(/\s+/u)
        .some((word) => word.startsWith(prefix)) ||
      (person.email?.startsWith(prefix) ?? false);
    const attending = new Set(
      this.rows.filter((r) => r.meetingId === meetingId).map((r) => r.personId),
    );
    const found = this.people.filter(matches).map((person) => ({
      person: { id: person.id, name: person.name },
      isAttendee: attending.has(person.id),
      isMe: person.isMe,
    }));
    return [...found.filter((c) => c.isAttendee), ...found.filter((c) => !c.isAttendee)].slice(
      0,
      8,
    );
  }

  rename(rename: Generated.Rename): CommandResult<Generated.Renamed> {
    const row = this.find(rename.meetingId, rename.speakerId);
    if (row === undefined) return failure(notFound(`speaker ${rename.speakerId}`));
    const person = this.resolve(rename.name);
    if ('code' in person) return failure(person);
    if (rename.scope.kind === 'segment')
      return this.renameSegment(row, rename.scope.segmentId, person);
    const enrolled = this.voiceprints.enrol(person, rename.meetingId, this.speechOf(row.speakerId));
    row.personId = person.id;
    row.suggestion = null;
    this.onChange(['speakers', 'speaker_suggestions', 'voiceprints']);
    return ok({ person: { id: person.id, name: person.name }, speakerId: row.speakerId, enrolled });
  }

  merge(meetingId: MeetingId, from: SpeakerId, into: SpeakerId): CommandResult<null> {
    const source = this.find(meetingId, from);
    const target = this.find(meetingId, into);
    if (source === undefined || target === undefined) return failure(notFound('speaker'));
    if (from === into)
      return failure({ code: 'invalid', message: 'a speaker cannot be merged into itself' });
    for (const said of this.said.values()) if (said.speakerId === from) said.speakerId = into;
    target.personId ??= source.personId;
    this.rows = this.rows.filter((row) => row.speakerId !== from);
    this.onChange(['speakers', 'transcript_segments']);
    return ok(null);
  }

  confirm(meetingId: MeetingId, speakerId: SpeakerId): CommandResult<Generated.Renamed> {
    const suggestion = this.find(meetingId, speakerId)?.suggestion;
    if (suggestion === null || suggestion === undefined) return failure(notFound('suggestion'));
    const name = { kind: 'person', personId: suggestion.personId } as const;
    return this.rename({ meetingId, speakerId, name, scope: { kind: 'all' } });
  }

  dismiss(speakerId: SpeakerId): CommandResult<null> {
    const row = this.rows.find((candidate) => candidate.speakerId === speakerId);
    if (row?.suggestion == null) return failure(notFound('suggestion'));
    row.suggestion = null;
    this.onChange(['speaker_suggestions']);
    return ok(null);
  }

  /** Leaves a speaker unnamed, optionally with a suggestion, for the demo. */
  unname(
    meetingId: MeetingId,
    label: string,
    suggest: { personId: PersonId; score: number } | null,
  ) {
    const row = this.rows.find((r) => r.meetingId === meetingId && r.label === label);
    if (row === undefined) return;
    row.personId = null;
    row.suggestion = suggest;
  }

  private renameSegment(row: Row, segmentId: Generated.SegmentId, person: MockPerson) {
    const said = this.said.get(segmentId);
    if (said?.meetingId !== row.meetingId) return failure<Generated.Renamed>(notFound('segment'));
    let target = this.rows.find((r) => r.meetingId === row.meetingId && r.personId === person.id);
    if (target === undefined) {
      const count = this.rows.filter((r) => r.meetingId === row.meetingId).length;
      target = {
        ...row,
        speakerId: crypto.randomUUID() as SpeakerId,
        label: `Speaker ${count + 1}`,
        color: (count % 6) + 1,
        isMe: false,
        personId: person.id,
        suggestion: null,
      };
      this.rows.push(target);
    }
    said.speakerId = target.speakerId;
    this.onChange(['speakers', 'transcript_segments']);
    return ok({
      person: { id: person.id, name: person.name },
      speakerId: target.speakerId,
      enrolled: false,
    });
  }

  private resolve(name: Generated.SpeakerName): MockPerson | AppError {
    if (name.kind === 'person') {
      return this.people.find((person) => person.id === name.personId) ?? notFound('person');
    }
    const trimmed = name.name.trim();
    if (trimmed === '' || trimmed.length > 100) {
      return { code: 'invalid', message: 'a name must be between 1 and 100 characters' };
    }
    const person: MockPerson = {
      id: crypto.randomUUID() as PersonId,
      name: trimmed,
      email: null,
      isMe: false,
    };
    this.people.push(person);
    this.onChange(['people']);
    return person;
  }

  private participant(row: Row, speech: number, total: number): Generated.Participant {
    const name = (id: PersonId) => this.people.find((person) => person.id === id)?.name ?? '';
    const suggestion = row.suggestion;
    return {
      speakerId: row.speakerId,
      label: row.label,
      color: row.color,
      isMe: row.isMe,
      person: row.personId === null ? null : { id: row.personId, name: name(row.personId) },
      speech,
      sharePercent: total === 0 ? 0 : Math.round((speech * 100) / total),
      suggestion:
        suggestion === null
          ? null
          : {
              person: { id: suggestion.personId, name: name(suggestion.personId) },
              score: suggestion.score,
            },
    };
  }

  private speechOf(speakerId: SpeakerId): number {
    let total = 0;
    for (const said of this.said.values()) if (said.speakerId === speakerId) total += said.length;
    return total;
  }

  private find(meetingId: MeetingId, speakerId: SpeakerId): Row | undefined {
    return this.rows.find((row) => row.meetingId === meetingId && row.speakerId === speakerId);
  }

  private load(fixture: FixtureMeeting) {
    const meetingId = fixture.id as MeetingId;
    const idOf = (label: string) => fixtureUuid(`speaker/${fixture.id}/${label}`) as SpeakerId;
    fixture.speakers.forEach((speaker, index) => {
      const person = this.people.find((p) => p.email !== null && p.email === speaker.attendee);
      this.rows.push({
        speakerId: idOf(speaker.label),
        meetingId,
        label: speaker.label,
        color: (index % 6) + 1,
        isMe: person?.isMe ?? false,
        personId: person?.id ?? null,
        suggestion: null,
      });
    });
    fixture.segments.forEach((segment, index) => {
      const id = fixtureUuid(`segment/${fixture.id}/${index}`) as Generated.SegmentId;
      this.said.set(id, {
        meetingId,
        speakerId: idOf(segment.speaker),
        length: segment.endMs - segment.startMs,
      });
    });
  }
}
