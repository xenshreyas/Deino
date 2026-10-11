/**
 * The transcript follows playback (TRN-03), as live mode follows new lines:
 * when the player moves on to a line that is off screen, the list scrolls
 * it into view, but only while the user is watching, that is, while the
 * line played before it was on screen. Someone who scrolled away to read
 * is not pulled back; the next line they scroll to picks following up again.
 */
import { useRef } from 'react';

import { usePlaybackListener } from '@/features/player';

import { rowPlayingAt } from '../model/playback';
import type { MeetingId, TranscriptRow } from '../types';
import type { TranscriptListController } from './use-transcript-list';

/** Whether `row` is inside the list's viewport. */
function isVisible(list: TranscriptListController, row: number): boolean {
  const element = list.scrollRef.current;
  const measured = list.virtualizer.measurementsCache[row];
  if (element === null || measured === undefined) return false;
  const top = element.scrollTop;
  return measured.end > top && measured.start < top + element.clientHeight;
}

/** Scrolls to each newly played row while the previous one was in view. */
export function usePlaybackFollow(
  meetingId: MeetingId,
  rows: readonly TranscriptRow[],
  list: TranscriptListController,
): void {
  // The listener is subscribed once; it reads the latest rows and list here.
  const latest = useRef({ rows, list });
  latest.current = { rows, list };
  const watched = useRef(-1);
  usePlaybackListener(meetingId, (ms) => {
    const { rows: now, list: view } = latest.current;
    const row = rowPlayingAt(now, ms);
    if (row < 0 || row === watched.current) return;
    const following = watched.current < 0 || isVisible(view, watched.current);
    watched.current = row;
    if (following && !isVisible(view, row)) {
      view.virtualizer.scrollToIndex(row, { align: 'center' });
    }
  });
}
