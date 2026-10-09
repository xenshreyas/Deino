/**
 * Starting a recording from the shell (sidebar, ⌘⇧R, palette, tray).
 *
 * The record screen (`/`) owns the recording lifecycle until FND-05 and
 * CAP-10 move it to Rust. It listens for {@link START_RECORDING_EVENT} while
 * mounted, and on mount it starts a recording if the session flag
 * {@link AUTO_START_FLAG} is set (`hooks/useRecordingStart.ts`). Both paths
 * ignore a request while a recording is already running.
 */

/** Window event the record screen listens for. */
export const START_RECORDING_EVENT = 'start-recording-from-sidebar';

/** `sessionStorage` key that makes the record screen start on mount. */
export const AUTO_START_FLAG = 'autoStartRecording';

/** Path of the record screen. */
const RECORD_PATH = '/';

/**
 * Starts a recording: directly when the record screen is showing, otherwise by
 * flagging an auto-start and opening it.
 */
export function requestRecordingStart(pathname: string, openRecordScreen: () => void): void {
  if (pathname === RECORD_PATH) {
    window.dispatchEvent(new CustomEvent(START_RECORDING_EVENT));
    return;
  }
  try {
    window.sessionStorage.setItem(AUTO_START_FLAG, 'true');
  } catch (error: unknown) {
    console.error('[shell] Could not flag the recording auto-start', error);
  }
  openRecordScreen();
}
