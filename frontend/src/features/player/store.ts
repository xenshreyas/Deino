/**
 * The player's shared state (Zustand): which meeting's recording is loaded,
 * where it is, and its controller. The dock writes it through the
 * controller's sink; the transcript (current line and word, "Play from
 * here", click to seek) and the citations' Play read it and drive the
 * controller through {@link playerActions}. One meeting page, one player.
 */
import { create } from 'zustand';

import type { PlaybackController, PlaybackSink } from './model/playback-controller';
import type { AudioSource, MeetingId, PlayerStatus } from './types';

/** What the store holds. */
export interface PlaybackState {
  /** The meeting whose recording is loaded, if any. */
  meetingId: MeetingId | null;
  controller: PlaybackController | null;
  status: PlayerStatus;
  playing: boolean;
  /** The user played or moved the recording: the transcript follows it. */
  engaged: boolean;
  /** The position, in milliseconds. */
  time: number;
  /** The length, in milliseconds (0 until known). */
  duration: number;
  rate: number;
  source: AudioSource | null;
}

/** The store's resting state: nothing loaded. */
const IDLE: PlaybackState = {
  meetingId: null,
  controller: null,
  status: 'loading',
  playing: false,
  engaged: false,
  time: 0,
  duration: 0,
  rate: 1,
  source: null,
};

/** The player's state. */
export const usePlaybackStore = create<PlaybackState>()(() => IDLE);

/** A sink that writes `meetingId`'s playback into the store. */
export function storeSink(meetingId: MeetingId): PlaybackSink {
  const set = (patch: Partial<PlaybackState>) => {
    if (usePlaybackStore.getState().meetingId === meetingId) usePlaybackStore.setState(patch);
  };
  return {
    status: (status) => {
      set({ status });
    },
    playing: (playing) => {
      set(playing ? { playing, engaged: true } : { playing });
    },
    time: (time) => {
      set({ time });
    },
    duration: (duration) => {
      set({ duration });
    },
    rate: (rate) => {
      set({ rate });
    },
  };
}

/** Makes `controller` the player of `meetingId`. */
export function loadPlayer(
  meetingId: MeetingId,
  controller: PlaybackController,
  source: AudioSource,
): void {
  usePlaybackStore.setState({ ...IDLE, meetingId, controller, source });
}

/** Forgets `meetingId`'s player (its dock unmounted). */
export function unloadPlayer(meetingId: MeetingId): void {
  if (usePlaybackStore.getState().meetingId === meetingId) usePlaybackStore.setState(IDLE);
}

/** The controller of `meetingId`, when its recording is loaded. */
function controllerOf(meetingId: MeetingId): PlaybackController | null {
  const state = usePlaybackStore.getState();
  return state.meetingId === meetingId ? state.controller : null;
}

/** What other features do to the player. Each is a no-op without a recording. */
export const playerActions = {
  /** Moves `meetingId`'s recording to `ms`, playing or not. */
  seek: (meetingId: MeetingId, ms: number): void => {
    const controller = controllerOf(meetingId);
    if (controller === null) return;
    usePlaybackStore.setState({ engaged: true });
    controller.seek(ms);
  },
  /** Moves to `ms` and plays. */
  playFrom: (meetingId: MeetingId, ms: number): void => {
    playerActions.seek(meetingId, ms);
    controllerOf(meetingId)?.play();
  },
};
