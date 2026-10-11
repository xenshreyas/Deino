/**
 * The live recorder's UI (FND-05b, CAP-02). Rust owns the recording: it
 * creates the meeting when recording starts, saves the transcript as it is
 * spoken, and moves the meeting to processing on stop. This feature only
 * asks and shows.
 *
 * | Export | Where |
 * |---|---|
 * | {@link RecordingSync} | once, in the app shell: state events, the stop hand-off, health toasts |
 * | {@link RecordingControls}, {@link LiveTranscript} | the record screen |
 * | {@link useRecordingState}, {@link useElapsed}, {@link useRecordingActions} | the sidebar's record block, anything that shows the recorder |
 * | {@link startRecording} | the shell's start paths (sidebar, ⌘⇧R, palette, detection) |
 */
export { recordingApi } from './api';
export { LiveTranscript } from './components/live-transcript';
export { RecordingControls } from './components/recording-controls';
export { RecordingSync } from './components/recording-sync';
export { recordingCopy } from './copy';
export { formatElapsed } from './elapsed';
export { useElapsed } from './hooks/use-elapsed';
export {
  IDLE,
  recordingKeys,
  recordingStateQuery,
  startRecording,
  useRecordingActions,
  useRecordingState,
} from './queries';
export type { RecordingPhase, RecordingSnapshot, TranscriptLine } from './types';
