/**
 * Typed calls to the `recording` plugin, the feature's only IPC. Rust owns
 * the recording (FND-05b, CAP-02): it creates the meeting, saves the
 * transcript as it is spoken and hands the meeting to the pipeline on stop.
 * The record panel (CAP-10) and auto-stop (CAP-07) answer through it too.
 */
import type { MeetingId } from '@/features/meetings';
import { commands } from '@/lib/ipc/generated/recording';
import { unwrap } from '@/lib/ipc/result';

import type { AutoStopState, Microphone, PanelEdge, PanelRegion, RecordingSnapshot } from './types';

export const recordingApi = {
  /** Starts recording a new meeting; resolves to its ID once audio flows. */
  start: (title: string | null): Promise<MeetingId> => unwrap(commands.start(title)),
  /** Stops, saves the last lines and starts the pipeline; resolves to the meeting. */
  stop: (): Promise<MeetingId> => unwrap(commands.stop()),
  /** Pauses capture; the recording skips the pause. */
  pause: (): Promise<RecordingSnapshot> => unwrap(commands.pause()),
  /** Resumes a paused recording. */
  resume: (): Promise<RecordingSnapshot> => unwrap(commands.resume()),
  /** The recorder's state now. */
  state: (): Promise<RecordingSnapshot> => unwrap(commands.state()),
  /** Records more of a finished meeting, appended at its end (CAP-07). */
  resumeMeeting: (meetingId: MeetingId): Promise<MeetingId> =>
    unwrap(commands.resumeMeeting(meetingId)),
  /** Switches the recording to the microphone named `name` (`null`: the default). */
  setMicrophone: (name: string | null): Promise<RecordingSnapshot> =>
    unwrap(commands.setMicrophone(name)),
  /** Records the call's audio, or stops it. */
  setSystemAudio: (enabled: boolean): Promise<RecordingSnapshot> =>
    unwrap(commands.setSystemAudio(enabled)),
  /** The Mac's microphones, the default first. */
  microphones: (): Promise<Microphone[]> => unwrap(commands.microphones()),
  /** "Keep recording": withdraws the ending notice. */
  keepRecording: (): Promise<AutoStopState> => unwrap(commands.keepRecording()),
  /** Answers "Start a new note?" for the meeting that began. */
  answerSplit: (accept: boolean): Promise<AutoStopState> => unwrap(commands.answerSplit(accept)),
  /** The ending notice and the offer on screen. */
  autoStop: (): Promise<AutoStopState> => unwrap(commands.autoStop()),
  /** Brings the main window forward on the live meeting. */
  openLiveMeeting: (): Promise<null> => unwrap(commands.openLiveMeeting()),
  /** Where the pill is inside the panel window (the rest lets clicks through); answers with the edge it hugs. */
  setPanelRegion: (region: PanelRegion): Promise<PanelEdge> =>
    unwrap(commands.setPanelRegion(region)),
};
