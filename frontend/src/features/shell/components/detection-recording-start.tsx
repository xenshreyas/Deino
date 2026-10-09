import { useRouter } from '@tanstack/react-router';
import { toast } from 'sonner';

import { useRecordingRequests } from '@/features/detection';

import { shellCopy } from '../copy';
import { requestRecordingStart } from '../recording';

interface DetectionRecordingStartProps {
  /** While onboarding runs, recording can't start; the user gets a toast. */
  onboarding: boolean;
}

/**
 * Starts a recording when meeting detection asks ("Take notes" on the
 * prompt, or the auto-start countdown), through the same path as the
 * sidebar, ⌘⇧R and the tray: the record screen owns the recording until
 * CAP-02 moves it to Rust. Rust has already brought the window forward for
 * "Take notes"; an auto-start records without taking focus.
 *
 * Renders nothing; the parent remounts it with a `key` when `onboarding` changes.
 */
export function DetectionRecordingStart({ onboarding }: DetectionRecordingStartProps) {
  const router = useRouter();
  useRecordingRequests(() => {
    if (onboarding) {
      toast.error(shellCopy.finishSetupTitle, { description: shellCopy.finishSetupDescription });
      return;
    }
    requestRecordingStart(router.state.location.pathname, () => void router.navigate({ to: '/' }));
  });
  return null;
}
