/**
 * Keeps the UI in step with the recorder, wherever the recording was
 * started or stopped (record screen, sidebar, menu bar, ⌘⇧R, detection):
 *
 * - each `recording-state` event becomes the cached state;
 * - when a recording finishes saving while the record screen shows, its
 *   meeting page opens;
 * - the record panel's "Open notes" (`open-live-meeting`) opens the record
 *   screen;
 * - health notices become toasts, and so does auto-stop's "Ending…" notice
 *   when the record panel is switched off (the panel shows it otherwise).
 *
 * Renders nothing; mount it once, inside the router.
 */
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useRouter } from '@tanstack/react-router';
import { useRef } from 'react';
import { toast } from 'sonner';

import { useSettings } from '@/features/settings';
import { useIpcEvent } from '@/lib/ipc';
import { events } from '@/lib/ipc/generated/recording';

import { recordingApi } from '../api';
import { endingReason, panelCopy, recordingCopy } from '../copy';
import { recordingKeys, recordingStateQuery } from '../queries';
import type { EndingNotice, HealthKind, HealthNotice, RecordingSnapshot } from '../types';

/** The record screen's path. */
const RECORD_PATH = '/';

/** The toast id of the "Ending…" notice, so a new one replaces the old. */
const ENDING_TOAST = 'recording-ending';

/**
 * Notices that report a recovery rather than a problem: the pill's warning
 * clears on its own, so they say nothing (UX-31).
 */
const GOOD_NEWS: ReadonlySet<HealthKind> = new Set(['recovered', 'awake']);

function showHealth({ kind }: HealthNotice): void {
  if (!GOOD_NEWS.has(kind)) toast.warning(recordingCopy.health[kind]);
}

/** The "Ending…" notice as a toast with "Keep recording". */
function showEnding(ending: EndingNotice | null): void {
  if (ending === null) {
    toast.dismiss(ENDING_TOAST);
    return;
  }
  toast.warning(endingReason[ending.reason], {
    id: ENDING_TOAST,
    description: panelCopy.ending(
      Math.max(0, Math.round((Date.parse(ending.stopAt) - Date.now()) / 1000)),
    ),
    duration: Math.max(1000, Date.parse(ending.stopAt) - Date.now()),
    action: { label: panelCopy.keepRecording, onClick: () => void recordingApi.keepRecording() },
  });
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
  const panelShown = useSettings().data?.recording.showPanel ?? true;
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
    if (!panelShown) showEnding(state.ending);
  });
  useIpcEvent(events.health, ({ notice }) => {
    showHealth(notice);
  });
  return null;
}
