import { useRouter } from '@tanstack/react-router';
import { listen } from '@tauri-apps/api/event';
import { toast } from 'sonner';

import { useMountEffect } from '@/hooks/useMountEffect';

import { shellCopy } from '../copy';
import { requestRecordingStart } from '../recording';
import { holdSubscriptions } from '../tauri-events';

/** Emitted by the Rust tray menu's "Start/Stop recording" item. */
const TRAY_TOGGLE_EVENT = 'request-recording-toggle';

interface TrayRecordingToggleProps {
  /** While onboarding runs, recording can't start; the user gets a toast. */
  onboarding: boolean;
}

/**
 * Forwards the tray's recording toggle to the record screen, opening it
 * first when another page is showing.
 *
 * Renders nothing. `onboarding` is read on mount, so the parent remounts this
 * with a `key` when it changes.
 */
export function TrayRecordingToggle({ onboarding }: TrayRecordingToggleProps) {
  const router = useRouter();
  useMountEffect(() =>
    holdSubscriptions([
      listen(TRAY_TOGGLE_EVENT, () => {
        if (onboarding) {
          toast.error(shellCopy.finishSetupTitle, {
            description: shellCopy.finishSetupDescription,
          });
          return;
        }
        requestRecordingStart(
          router.state.location.pathname,
          () => void router.navigate({ to: '/' }),
        );
      }),
    ]),
  );
  return null;
}
