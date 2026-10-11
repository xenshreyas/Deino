/**
 * A meeting's transcript as rows: the saved transcript (TanStack Query,
 * refreshed by `DbChanged`) overlaid with the recorder's `live-segment`
 * events since this mounted, grouped into speaker turns. Remount with `key`
 * for another meeting.
 */
import { useQuery } from '@tanstack/react-query';
import { useMemo, useState } from 'react';

import { useIpcEvent } from '@/lib/ipc';
import { events } from '@/lib/ipc/generated/recording';

import { applyUpdate, type LiveLines, mergeLines } from '../model/lines';
import { buildRows } from '../model/rows';
import { SpeakerDirectory } from '../model/speakers';
import { transcriptQuery } from '../queries';
import type { MeetingId, SpeakerView, TranscriptRow } from '../types';

/** The transcript's state, as the panel renders it. */
export type TranscriptState =
  | { status: 'pending' }
  | { status: 'error'; retry: () => void }
  | { status: 'ready'; rows: TranscriptRow[]; speakers: SpeakerView[] };

/** The rows of `meetingId`, live lines included. */
export function useTranscript(meetingId: MeetingId): TranscriptState {
  const query = useQuery(transcriptQuery(meetingId));
  const [live, setLive] = useState<LiveLines>(() => new Map());
  useIpcEvent(events.liveSegment, ({ update }) => {
    if (update.meetingId !== meetingId) return;
    setLive((current) => applyUpdate(current, update));
  });
  const transcript = query.data;
  const directory = useMemo(() => new SpeakerDirectory(transcript?.speakers ?? []), [transcript]);
  const rows = useMemo(
    () => buildRows(mergeLines(transcript?.segments ?? [], live), directory),
    [transcript, live, directory],
  );
  if (transcript !== undefined) return { status: 'ready', rows, speakers: directory.all };
  if (query.isError) return { status: 'error', retry: () => void query.refetch() };
  return { status: 'pending' };
}
