/**
 * Builders for the transcript feature's tests: segments, speakers and
 * live updates with readable defaults. Test-only; nothing imports it at
 * run time.
 */
import type {
  MeetingId,
  Segment,
  SegmentId,
  SegmentUpdate,
  SpeakerId,
  TranscriptSpeaker,
} from './types';

export const MEETING = 'meeting-test' as MeetingId;

/** A speaker; the user's own voice with `isMe`. */
export function speaker(id: string, name: string, extra: Partial<TranscriptSpeaker> = {}) {
  const base: TranscriptSpeaker = {
    id: id as SpeakerId,
    name,
    personId: null,
    isMe: false,
    color: 2,
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
