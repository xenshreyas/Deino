/**
 * Typed calls to the `recording` plugin, the feature's only IPC. Rust owns
 * the recording (FND-05b, CAP-02): it creates the meeting, saves the
 * transcript as it is spoken and hands the meeting to the pipeline on stop.
 */
import type { MeetingId } from '@/features/meetings';
import { commands } from '@/lib/ipc/generated/recording';
import { unwrap } from '@/lib/ipc/result';

import type { RecordingSnapshot, Segment } from './types';

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
  /** The meeting's saved transcript, oldest first. */
  transcript: (meetingId: MeetingId): Promise<Segment[]> => unwrap(commands.transcript(meetingId)),
};
