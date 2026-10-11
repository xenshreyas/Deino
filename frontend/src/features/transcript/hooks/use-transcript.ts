/**
 * A meeting's transcript as rows: the saved transcript (TanStack Query,
 * refreshed by `DbChanged`) overlaid with the recorder's `live-segment`
 * events since this mounted, its lines attributed to the meeting's
 * participants (CAP-04) and grouped into speaker turns. Remount with `key`
 * for another meeting.
 */
import { useQuery } from '@tanstack/react-query';
import { useMemo, useState } from 'react';

import { useParticipants } from '@/features/speakers';
import { useIpcEvent } from '@/lib/ipc';
import { events } from '@/lib/ipc/generated/recording';

import { applyUpdate, type LiveLines, mergeLines } from '../model/lines';
import { buildRows } from '../model/rows';
import { SpeakerDirectory } from '../model/speakers';
import { transcriptQuery } from '../queries';
import type { MeetingId, Participant, SpeakerView, TranscriptRow } from '../types';

/** The transcript's state, as the panel renders it. */
export type TranscriptState =
  | { status: 'pending' }
  | { status: 'error'; retry: () => void }
  | { status: 'ready'; rows: TranscriptRow[]; speakers: SpeakerView[] };

/** No participants: lines fall back to Me and Them by channel. */
const NO_PARTICIPANTS: readonly Participant[] = [];

/** The rows of `meetingId`, live lines included. */
export function useTranscript(meetingId: MeetingId): TranscriptState {
  const query = useQuery(transcriptQuery(meetingId));
  const participants = useParticipants(meetingId);
  const [live, setLive] = useState<LiveLines>(() => new Map());
  useIpcEvent(events.liveSegment, ({ update }) => {
    if (update.meetingId !== meetingId) return;
    setLive((current) => applyUpdate(current, update));
  });
  const transcript = query.data;
  const people = participants.data ?? NO_PARTICIPANTS;
  const directory = useMemo(() => new SpeakerDirectory(people), [people]);
  const rows = useMemo(
    () => buildRows(mergeLines(transcript?.segments ?? [], live), directory),
    [transcript, live, directory],
  );
  // Wait for the names too (a failed read falls back to Me and Them), so
  // names never flash from "Them" to a person.
  if (transcript !== undefined && !participants.isPending) {
    return { status: 'ready', rows, speakers: directory.all };
  }
  if (query.isError) return { status: 'error', retry: () => void query.refetch() };
  return { status: 'pending' };
}
