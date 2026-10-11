/**
 * A meeting's transcript as it is spoken: the saved segments, overlaid with
 * the `live-segment` events since this mounted. Remount with `key` for
 * another meeting.
 */
import { useQuery } from '@tanstack/react-query';
import { useMemo, useState } from 'react';

import type { MeetingId } from '@/features/meetings';
import { useIpcEvent } from '@/lib/ipc';
import { events } from '@/lib/ipc/generated/recording';

import { applyUpdate, type LiveLines, mergeLines } from '../lines';
import { recordingTranscriptQuery } from '../queries';
import type { TranscriptLine } from '../types';

/** The visible lines of `meetingId`, in spoken order. */
export function useLiveLines(meetingId: MeetingId): TranscriptLine[] {
  const saved = useQuery(recordingTranscriptQuery(meetingId)).data;
  const [live, setLive] = useState<LiveLines>(() => new Map());
  useIpcEvent(events.liveSegment, ({ update }) => {
    if (update.meetingId !== meetingId) return;
    setLive((current) => applyUpdate(current, update));
  });
  return useMemo(() => mergeLines(saved ?? [], live), [saved, live]);
}
