/**
 * Keeps the UI in step with the recorder, wherever the recording was
 * started or stopped (record screen, sidebar, menu bar, ⌘⇧R, detection):
 *
 * - each `recording-state` event becomes the cached state;
 * - when a recording finishes saving while the record screen shows, its
 *   meeting page opens;
 * - the record panel's "Open notes" (`open-live-meeting`) opens the record
 *   screen;
 * - health notices become toasts (auto-stop's "Ending…" notice has its own
 *   top-centre card, which Rust shows).
 *
 * Renders nothing; mount it once, inside the router.
 */
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useRouter } from '@tanstack/react-router';
import { useRef } from 'react';
import { toast } from 'sonner';

import { useIpcEvent } from '@/lib/ipc';
import { events } from '@/lib/ipc/generated/recording';

import { recordingCopy } from '../copy';
import { recordingKeys, recordingStateQuery } from '../queries';
import type { HealthKind, HealthNotice, RecordingSnapshot } from '../types';

/** The record screen's path. */
const RECORD_PATH = '/';

/**
 * Notices that report a recovery rather than a problem: the pill's warning
 * clears on its own, so they say nothing (UX-31).
 */
const GOOD_NEWS: ReadonlySet<HealthKind> = new Set(['recovered', 'awake']);

function showHealth({ kind }: HealthNotice): void {
  if (!GOOD_NEWS.has(kind)) toast.warning(recordingCopy.health[kind]);
}

/** The meeting that just finished saving, if `next` ends a stop. */
function finished(previous: RecordingSnapshot | undefined, next: RecordingSnapshot) {
  if (previous?.phase !== 'stopping' || next.phase !== 'idle') return null;
  return previous.meetingId;
}

/** The recorder's events, applied to the cache and the window. */
export function RecordingSync() {
  const client = useQueryClient();
  const router = useRouter();
  useQuery(recordingStateQuery());
  const last = useRef<RecordingSnapshot | undefined>(undefined);
  useIpcEvent(events.recordingState, ({ snapshot }) => {
    const meetingId = finished(last.current, snapshot);
    last.current = snapshot;
    client.setQueryData(recordingKeys.state(), snapshot);
    if (meetingId === null || router.state.location.pathname !== RECORD_PATH) return;
    void router.navigate({ to: '/meetings/$meetingId', params: { meetingId } });
  });
  useIpcEvent(events.openLiveMeeting, () => {
    void router.navigate({ to: '/' });
  });
  useIpcEvent(events.autoStopChanged, ({ state }) => {
    client.setQueryData(recordingKeys.autoStop(), state);
  });
  useIpcEvent(events.health, ({ notice }) => {
    showHealth(notice);
  });
  return null;
}
