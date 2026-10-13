/**
 * The record panel's page (CAP-10): the pill, wired to the recorder, and
 * auto-stop's offer and at most one consent reminder under it (CAP-07,
 * CAP-13 with CAP-09's screen capture folded in, UX-28). The window is transparent and a
 * little larger than the pill; the pill hugs the screen edge the panel is
 * snapped to and grows away from it. During the first recording, PRV-01's
 * "Pause for sensitive discussions" tip sits under the pill once.
 */
import { useQueryClient } from '@tanstack/react-query';
import { isTauri } from '@tauri-apps/api/core';
import { useRef, type MutableRefObject } from 'react';

import { PauseTipSlot } from '@/features/retention';
import { captureMenuItems, type CaptureSource } from '@/features/screen';
import { useIpcEvent } from '@/lib/ipc';
import { events } from '@/lib/ipc/generated/recording';
import { cn } from '@/ui/cn';

import { recordingApi } from '../../api';
import { issueLabel } from '../../copy';
import { formatElapsed } from '../../elapsed';
import { useAutoStop, useAutoStopAnswers } from '../../hooks/use-auto-stop';
import { useDeviceControls } from '../../hooks/use-device-controls';
import { useElapsed } from '../../hooks/use-elapsed';
import {
  usePanelHover,
  usePanelLevels,
  usePanelRegion,
  usePanelTheme,
  usePressOrDrag,
} from '../../hooks/use-record-panel';
import { showPillMenu } from '../../panel-window';
import { recordingKeys, useRecordingActions, useRecordingState } from '../../queries';
import type { AutoStopState, RecordingSnapshot } from '../../types';
import { Pill, type PillView } from './pill';
import type { PillActions } from './pill-controls';
import { RecordingReminder } from './reminder';
import { SplitCard } from './split-card';

/** Keeps the cached recorder state current (the panel has no `RecordingSync`). */
function useFollowRecorder(): RecordingSnapshot {
  const client = useQueryClient();
  useIpcEvent(events.recordingState, ({ snapshot }) => {
    client.setQueryData(recordingKeys.state(), snapshot);
  });
  return useRecordingState();
}

/** Opens the live meeting in the main window. */
function openNotes(): void {
  recordingApi.openLiveMeeting().catch((error: unknown) => {
    console.error('[recording] Could not open the live meeting', error);
  });
}

/**
 * Pops up the pill's ⋯ menu (UX-29): the microphone, the screen to
 * capture (remembering the pick) and whether the call's audio is recorded.
 */
async function openMoreMenu(
  snapshot: RecordingSnapshot,
  picked: MutableRefObject<CaptureSource | null>,
  devices: ReturnType<typeof useDeviceControls>,
): Promise<void> {
  if (!isTauri() || snapshot.phase === 'idle' || snapshot.meetingId === null) return;
  const [microphones, screens] = await Promise.all([
    recordingApi.microphones(),
    captureMenuItems(snapshot.meetingId, picked.current, (source) => {
      picked.current = source;
    }),
  ]);
  await showPillMenu({
    microphones,
    microphone: snapshot.microphone,
    onMicrophone: (name) => {
      devices.microphone.mutate(name);
    },
    screens,
    systemAudio: snapshot.systemAudio,
    onSystemAudio: () => {
      devices.systemAudio.mutate(!snapshot.systemAudio);
    },
  });
}

/** The pill's actions over the recorder and the devices. */
function usePillActions(snapshot: RecordingSnapshot): PillActions {
  const recorder = useRecordingActions();
  const picked = useRef<CaptureSource | null>(null);
  const devices = useDeviceControls();
  return {
    onPause: () => {
      recorder.pause.mutate();
    },
    onResume: () => {
      recorder.resume.mutate();
    },
    onStop: () => {
      recorder.stop.mutate();
    },
    onMore: () => {
      openMoreMenu(snapshot, picked, devices).catch((error: unknown) => {
        console.error('[recording] Could not open the pill menu', error);
      });
    },
    onOpenNotes: openNotes,
  };
}

/** What the pill shows now. */
function usePillView(
  snapshot: RecordingSnapshot,
  autoStop: AutoStopState,
  expanded: boolean,
): PillView {
  const seconds = useElapsed(snapshot);
  const levels = usePanelLevels();
  const issue = snapshot.issues[0];
  return {
    phase: snapshot.phase,
    elapsed: formatElapsed(seconds),
    levels,
    expanded,
    warning: issue === undefined ? null : issueLabel(issue),
  };
}

/** Auto-stop's back-to-back offer, while there is one. */
function NextMeetingOffer({ autoStop }: { autoStop: AutoStopState }) {
  const answers = useAutoStopAnswers();
  if (autoStop.split === null) return null;
  return (
    <SplitCard
      offer={autoStop.split}
      busy={answers.split.isPending}
      onAccept={() => {
        answers.split.mutate(true);
      }}
      onDismiss={() => {
        answers.split.mutate(false);
      }}
    />
  );
}

/** The record panel. */
export function RecordPanel() {
  usePanelTheme();
  const snapshot = useFollowRecorder();
  const autoStop = useAutoStop();
  const hover = usePanelHover();
  const stack = useRef<HTMLDivElement>(null);
  const edge = usePanelRegion(stack);
  const actions = usePillActions(snapshot);
  const view = usePillView(snapshot, autoStop, hover.expanded);
  const onPointerDown = usePressOrDrag(openNotes);
  return (
    <main
      className={cn(
        'flex h-screen w-screen p-3',
        edge === 'right' ? 'justify-end' : 'justify-start',
      )}
    >
      <div
        ref={stack}
        onPointerEnter={hover.onPointerEnter}
        onPointerLeave={hover.onPointerLeave}
        className={cn('flex h-max flex-col gap-2', edge === 'right' ? 'items-end' : 'items-start')}
      >
        {snapshot.phase === 'idle' ? null : (
          <Pill view={view} actions={actions} onPointerDown={onPointerDown} />
        )}
        <NextMeetingOffer autoStop={autoStop} />
        {snapshot.phase === 'recording' && autoStop.split === null ? <PauseTipSlot /> : null}
        <RecordingReminder meetingId={snapshot.phase === 'idle' ? null : snapshot.meetingId} />
      </div>
    </main>
  );
}
