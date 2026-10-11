/**
 * Voice profiles for `pnpm dev:mock`: an in-memory twin of
 * `deino_core::voiceprints` (enrol on a rename of all segments unless the
 * voice is too thin, keep each person's five clearest samples, delete one
 * or forget a voice). No vectors: the mock only keeps what Settings shows.
 */
import type * as Generated from '@/lib/ipc/generated/speakers';
import type { Table } from '@/lib/ipc/generated/shared';
import type { CommandResult } from '@/lib/ipc/result';

import { failure, ok } from './command-result';
import type { MockPerson } from './mock-speakers';

/** Same as `deino_core::voiceprints::MIN_ENROL_SPEECH`, in ms. */
const MIN_ENROL_SPEECH = 3_000;
/** Same as `deino_core::voiceprints::quality::FULL_QUALITY_SPEECH`, in ms. */
const FULL_QUALITY_SPEECH = 60_000;
/** Same as `deino_core::voiceprints::MAX_VOICEPRINTS_PER_PERSON`. */
const MAX_PER_PERSON = 5;

/** One stored sample. */
interface Sample extends Generated.VoiceprintSummary {
  personId: Generated.PersonId;
}

/** A stored sample as Settings lists it. */
function summaryOf(sample: Sample): Generated.VoiceprintSummary {
  return {
    id: sample.id,
    sourceMeetingId: sample.sourceMeetingId,
    sourceMeetingTitle: sample.sourceMeetingTitle,
    quality: sample.quality,
    createdAt: sample.createdAt,
  };
}

/** Every person's voice samples. */
export class MockVoiceprints {
  private samples: Sample[] = [];
  /** Meeting titles, for "From Standup". */
  readonly titles = new Map<string, string>();

  constructor(
    private readonly people: readonly MockPerson[],
    private readonly onChange: (tables: Table[]) => void,
  ) {}

  /** Saves a sample of `person` heard for `speech` ms; whether it was kept. */
  enrol(person: MockPerson, meetingId: Generated.MeetingId, speech: number): boolean {
    if (speech < MIN_ENROL_SPEECH) return false;
    const sample: Sample = {
      id: crypto.randomUUID() as Generated.VoiceprintId,
      personId: person.id,
      sourceMeetingId: meetingId,
      sourceMeetingTitle: this.titles.get(meetingId) ?? null,
      quality: Math.min(1, speech / FULL_QUALITY_SPEECH),
      createdAt: new Date().toISOString(),
    };
    this.samples.push(sample);
    const theirs = this.samples
      .filter((candidate) => candidate.personId === person.id)
      .sort((a, b) => b.quality - a.quality || b.createdAt.localeCompare(a.createdAt));
    const evicted = new Set(theirs.slice(MAX_PER_PERSON).map((candidate) => candidate.id));
    this.samples = this.samples.filter((candidate) => !evicted.has(candidate.id));
    return !evicted.has(sample.id);
  }

  /** Everyone with samples, by name, best sample first. */
  profiles(): Generated.VoiceProfile[] {
    return this.people
      .map((person) => ({
        personId: person.id,
        name: person.name,
        isMe: person.isMe,
        voiceprints: this.samples
          .filter((sample) => sample.personId === person.id)
          .sort((a, b) => b.quality - a.quality)
          .map(summaryOf),
      }))
      .filter((profile) => profile.voiceprints.length > 0)
      .sort((a, b) => a.name.localeCompare(b.name));
  }

  delete(id: Generated.VoiceprintId): CommandResult<null> {
    const before = this.samples.length;
    this.samples = this.samples.filter((sample) => sample.id !== id);
    if (this.samples.length === before) {
      return failure({ code: 'not_found', message: `voiceprint ${id} was not found` });
    }
    this.onChange(['voiceprints']);
    return ok(null);
  }

  forget(personId: Generated.PersonId): CommandResult<null> {
    this.samples = this.samples.filter((sample) => sample.personId !== personId);
    this.onChange(['voiceprints']);
    return ok(null);
  }
}
