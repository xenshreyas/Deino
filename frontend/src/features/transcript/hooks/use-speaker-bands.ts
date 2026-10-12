/**
 * Who spoke when in a meeting, for the player's scrubber (TRN-03): the saved
 * lines with their speakers' palette colours, from the same transcript and
 * participants the view reads (so they are cached once).
 */
import { useMemo } from 'react';

import type { SpeakerBand } from '@/features/player';

import { speakerBands } from '../model/playback';
import type { MeetingId } from '../types';
import { useTranscript } from './use-transcript';

/** No bands while the transcript loads. */
const NONE: readonly SpeakerBand[] = [];

/** The meeting's speaker bands; empty until its transcript is loaded. */
export function useSpeakerBands(meetingId: MeetingId): readonly SpeakerBand[] {
  const state = useTranscript(meetingId);
  const rows = state.status === 'ready' ? state.rows : undefined;
  return useMemo(() => (rows === undefined ? NONE : speakerBands(rows)), [rows]);
}
