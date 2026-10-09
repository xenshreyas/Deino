import { listen } from '@tauri-apps/api/event';
import { toast } from 'sonner';

import { useMountEffect } from '@/hooks/useMountEffect';

import { shellCopy } from '../copy';
import { holdSubscriptions } from '../tauri-events';

/** Emitted by the Rust tray menu's "Start/Stop recording" item. */
const TRAY_TOGGLE_EVENT = 'request-recording-toggle';

/** Window event `useRecordingStart` listens for to start a recording. */
const START_RECORDING_EVENT = 'start-recording-from-sidebar';

interface TrayRecordingToggleProps {
  /** While onboarding runs, recording can't start; the user gets a toast. */
  onboarding: boolean;
}

/** Reacts to one tray toggle request. */
function handleTrayToggle(onboarding: boolean): void {
  if (onboarding) {
    toast.error(shellCopy.finishSetupTitle, { description: shellCopy.finishSetupDescription });
    return;
  }
  window.dispatchEvent(new CustomEvent(START_RECORDING_EVENT));
}

/**
 * Forwards the tray's recording toggle to the record screen.
 *
 * Renders nothing. `onboarding` is read on mount, so the parent remounts this
 * with a `key` when it changes.
 */
export function TrayRecordingToggle({ onboarding }: TrayRecordingToggleProps) {
  useMountEffect(() =>
    holdSubscriptions([listen(TRAY_TOGGLE_EVENT, () => handleTrayToggle(onboarding))]),
  );
  return null;
}
