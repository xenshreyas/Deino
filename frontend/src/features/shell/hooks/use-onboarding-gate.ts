import { invoke } from '@tauri-apps/api/core';
import { useCallback, useState } from 'react';
import { z } from 'zod';

import { useMountEffect } from '@/hooks/useMountEffect';

/** The part of the Rust `OnboardingStatus` the gate reads; `null` before first run. */
const onboardingStatusSchema = z.object({ completed: z.boolean() }).nullable();

/** State of the first-run gate in front of the whole app. */
export interface OnboardingGate {
  /** True while the onboarding flow should replace the app. */
  showOnboarding: boolean;
  /** Leaves onboarding and reloads so every provider starts from fresh state. */
  completeOnboarding: () => void;
}

/** Asks the Rust core whether onboarding has been completed. */
async function readOnboardingCompleted(): Promise<boolean> {
  const status = onboardingStatusSchema.parse(await invoke('get_onboarding_status'));
  return status?.completed ?? false;
}

/**
 * Decides whether to show onboarding, checking once on mount.
 *
 * The app renders while the check runs, as before the Vite migration. If the
 * status can't be read, onboarding is shown so setup can repair it.
 */
export function useOnboardingGate(): OnboardingGate {
  const [showOnboarding, setShowOnboarding] = useState(false);

  useMountEffect(() => {
    readOnboardingCompleted()
      .then((completed) => setShowOnboarding(!completed))
      .catch((error: unknown) => {
        console.error('[shell] Failed to read onboarding status', error);
        setShowOnboarding(true);
      });
  });

  const completeOnboarding = useCallback(() => {
    setShowOnboarding(false);
    window.location.reload();
  }, []);

  return { showOnboarding, completeOnboarding };
}
