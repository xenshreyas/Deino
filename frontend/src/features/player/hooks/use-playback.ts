/**
 * Read the player from anywhere on the meeting page: whether a meeting's
 * recording can be played, and values derived from its position that
 * re-render only when they change (the transcript's current line changes a
 * few times a minute, though the position changes every frame).
 */
import { useMountEffect } from '@/hooks/useMountEffect';

import { type PlaybackState, usePlaybackStore } from '../store';
import type { MeetingId } from '../types';

/** Played, or moved off the start: the transcript shows where it is. */
function engaged(state: PlaybackState): boolean {
  return state.engaged || state.time > 0;
}

/** True when `meetingId`'s recording is loaded in the player and playable. */
export function usePlayerReady(meetingId: MeetingId): boolean {
  return usePlaybackStore((state) => state.meetingId === meetingId && state.status !== 'error');
}

/**
 * `select(position)` for `meetingId` once its recording has been played or
 * moved; `null` before that, or for another meeting. Re-renders only
 * when the selected value changes (compare primitives).
 */
export function usePlaybackValue<T>(meetingId: MeetingId, select: (ms: number) => T): T | null {
  return usePlaybackStore((state) =>
    state.meetingId === meetingId && engaged(state) ? select(state.time) : null,
  );
}

/**
 * Calls `onTime` with `meetingId`'s position as it changes, while the
 * component is mounted, without re-rendering it (for scrolling to follow
 * playback). `onTime` should read anything it needs through a ref.
 */
export function usePlaybackListener(meetingId: MeetingId, onTime: (ms: number) => void): void {
  useMountEffect(() =>
    usePlaybackStore.subscribe((state, previous) => {
      if (state.meetingId !== meetingId || !engaged(state)) return;
      if (state.time !== previous.time) onTime(state.time);
    }),
  );
}
