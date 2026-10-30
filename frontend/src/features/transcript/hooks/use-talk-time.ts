/**
 * Who talked how much in a meeting, for the meeting page's context rail:
 * the same transcript rows the view and the player's bands read (so they
 * are cached once), live lines included once they finish.
 */
import { useMemo } from 'react';

import { type TalkShare, talkShares } from '../model/talk-time';
import type { MeetingId } from '../types';
import { useTranscript } from './use-transcript';

/** No shares while the transcript loads. */
const NONE: readonly TalkShare[] = [];

/** The meeting's talk time per speaker, most first; empty until its transcript is loaded. */
export function useTalkTime(meetingId: MeetingId): readonly TalkShare[] {
  const state = useTranscript(meetingId);
  const rows = state.status === 'ready' ? state.rows : undefined;
  return useMemo(() => (rows === undefined ? NONE : talkShares(rows)), [rows]);
}
