/**
 * Starting a recording from the shell (sidebar, ⌘⇧R, palette, the meetings
 * list). Rust owns the recording (FND-05b, CAP-02): the shell opens the
 * record screen, which shows the meeting being recorded, and asks Rust to
 * start. While a recording runs, the request only opens the screen.
 */
import { startRecording } from '@/features/recording';

/** Opens the record screen and starts a recording. */
export function requestRecordingStart(openRecordScreen: () => void): Promise<void> {
  openRecordScreen();
  return startRecording();
}
