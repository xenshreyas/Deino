/**
 * The live recorder's UI (FND-05b, CAP-02). Rust owns the recording: it
 * creates the meeting when recording starts, saves the transcript as it is
 * spoken, and moves the meeting to processing on stop. This feature only
 * asks and shows.
 *
 * | Export | Where |
 * |---|---|
 * | {@link RecordingSync} | once, in the app shell: state events, the stop hand-off, health toasts |
 * | {@link RecordingControls} | the record screen (its live transcript is `features/transcript`) |
 * | {@link useRecordingState}, {@link useElapsed}, {@link useRecordingActions} | the sidebar's record block, anything that shows the recorder |
 * | {@link startRecording} | the shell's start paths (sidebar, ⌘⇧R, palette, detection) |
 * | {@link RecordPanel} | the floating record panel's page (`record-panel.html`, CAP-10) |
 * | {@link ResumeRecordingButton} | the meeting header: record more of a finished meeting (CAP-07) |
 */
export { recordingApi } from './api';
export { EndingCard } from './components/ending-card';
export { RecordPanel } from './components/record-panel/record-panel';
export { RecordingControls } from './components/recording-controls';
export { RecordingSync } from './components/recording-sync';
export { ResumeRecordingButton } from './components/resume-recording';
export { issueLabel, recordingCopy } from './copy';
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
export type { HealthIssue, RecordingPhase, RecordingSnapshot } from './types';
