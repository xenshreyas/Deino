/**
 * The record panel's and auto-stop's half of the `recording` mock
 * (CAP-10, CAP-07): device switches, the microphone list, the "Ending…"
 * notice and the "new note" offer, and a few demo scenes the panel page
 * shows with `?demo=…` in `pnpm dev:mock` (for screenshots and design).
 */
import type * as Generated from '@/lib/ipc/generated/recording';

import { failure, ok } from './command-result';
import { latency } from './latency';
import { mockEvent } from './mock-event';

/** The recorder state the panel mock reads and changes. */
export interface SnapshotStore {
  get: () => Generated.RecordingSnapshot;
  publish: (next: Generated.RecordingSnapshot) => Generated.RecordingSnapshot;
}

export const panelEvents = {
  audioLevel: mockEvent<Generated.AudioLevel>('plugin:recording:audio-level'),
  autoStopChanged: mockEvent<Generated.AutoStopChanged>('plugin:recording:auto-stop-changed'),
  openLiveMeeting: mockEvent<Generated.OpenLiveMeeting>('plugin:recording:open-live-meeting'),
  panelPointer: mockEvent<Generated.PanelPointer>('plugin:recording:panel-pointer'),
  panelSide: mockEvent<Generated.PanelSide>('plugin:recording:panel-side'),
};

const MICROPHONES: Generated.Microphone[] = [
  { name: 'MacBook Pro Microphone', isDefault: true, bluetooth: false },
  { name: 'AirPods Pro', isDefault: false, bluetooth: true },
  { name: 'Shure MV7', isDefault: false, bluetooth: false },
];

let autoStop: Generated.AutoStopState = { ending: null, split: null };

/** Shows `next` as auto-stop's state. */
export function setAutoStop(next: Generated.AutoStopState): Generated.AutoStopState {
  autoStop = next;
  void panelEvents.autoStopChanged.emit({ state: next });
  return next;
}

const conflict = () => failure<never>({ code: 'conflict', message: 'nothing is being recorded' });

/** The panel's commands over `store`. */
export function panelCommands(store: SnapshotStore) {
  const running = () => store.get().phase === 'recording' || store.get().phase === 'paused';
  return {
    async setMicrophone(name: string | null) {
      await latency();
      return running() ? ok(store.publish({ ...store.get(), microphone: name })) : conflict();
    },
    async setSystemAudio(enabled: boolean) {
      await latency();
      return running() ? ok(store.publish({ ...store.get(), systemAudio: enabled })) : conflict();
    },
    async microphones() {
      await latency();
      return ok(MICROPHONES);
    },
    async keepRecording() {
      await latency();
      return ok(setAutoStop({ ...autoStop, ending: null }));
    },
    async answerSplit() {
      await latency();
      return ok(setAutoStop({ ...autoStop, split: null }));
    },
    async autoStop() {
      await latency();
      return ok(autoStop);
    },
    async openLiveMeeting() {
      await latency();
      const meetingId = store.get().meetingId;
      if (meetingId === null) return conflict();
      void panelEvents.openLiveMeeting.emit({ meetingId });
      return ok(null);
    },
    async setPanelRegion() {
      await latency();
      return ok<Generated.PanelEdge>('right');
    },
  };
}
