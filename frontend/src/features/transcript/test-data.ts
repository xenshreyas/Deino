/**
 * Builders for the transcript feature's tests: segments, speakers and
 * live updates with readable defaults. Test-only; nothing imports it at
 * run time.
 */
import type { PersonId } from '@/lib/ipc/generated/speakers';

import type { MeetingId, Participant, Segment, SegmentId, SegmentUpdate, SpeakerId } from './types';

export const MEETING = 'meeting-test' as MeetingId;

/**
 * A participant named `name` (a person); `null` leaves the voice unnamed
 * ("Speaker 2"). The user's own voice with `{ isMe: true }`.
 */
export function speaker(id: string, name: string | null, extra: Partial<Participant> = {}) {
  const base: Participant = {
    speakerId: id as SpeakerId,
    label: 'Speaker 2',
    color: 2,
    isMe: false,
    person: name === null ? null : { id: `person-${id}` as PersonId, name },
    speech: 0,
    sharePercent: 0,
    suggestion: null,
  };
  return { ...base, ...extra };
}

/** A saved segment said by `speakerId` at `start` seconds. */
export function segment(id: string, start: number, text: string, speakerId: string | null) {
  const saved: Segment = {
    id: id as SegmentId,
    meetingId: MEETING,
    channel: 'system',
    speakerId: speakerId as SpeakerId | null,
    start: start * 1000,
    end: start * 1000 + 900,
    text,
    words: [],
    isDeleted: false,
    editedAt: null,
  };
  return saved;
}

/** A live update of line `id`. */
export function liveUpdate(id: string, state: SegmentUpdate['state'], text = 'hello') {
  const update: SegmentUpdate = {
    meetingId: MEETING,
    segmentId: id as SegmentId,
    channel: 'mic',
    start: 500,
    end: 500,
    text,
    state,
  };
  return update;
}
