import { useCallback, useState } from 'react';

import { settingsApi } from '@/features/settings';
import { useMountEffect } from '@/hooks/useMountEffect';

/** State of the first-run gate in front of the whole app. */
export interface OnboardingGate {
  /** True while the onboarding flow should replace the app. */
  showOnboarding: boolean;
  /** Leaves onboarding and reloads so every provider starts from fresh state. */
  completeOnboarding: () => void;
}

/** Asks the typed settings whether onboarding has been completed. */
async function readOnboardingCompleted(): Promise<boolean> {
  const { onboarding } = await settingsApi.get();
  return onboarding.completed;
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
      .then((completed) => {
        setShowOnboarding(!completed);
      })
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
