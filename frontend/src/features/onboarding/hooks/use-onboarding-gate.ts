import { useSettings } from '@/features/settings';

import { isSetupStep } from '../machine';

/** What the shell renders in front of the routes. */
export type OnboardingGate =
  /** Settings are loading: render nothing yet, so neither screen flashes. */
  | 'pending'
  /** Setup is not finished: show onboarding instead of the app. */
  | 'onboarding'
  /** Show the app. */
  | 'app';

/**
 * First launch routes into onboarding; anyone who finished (or skipped)
 * setup never sees it again unless Settings resumes it. If the settings
 * cannot be read the app opens, since onboarding could not save either.
 */
export function useOnboardingGate(): OnboardingGate {
  const settings = useSettings();
  if (settings.isError) return 'app';
  if (settings.data === undefined) return 'pending';
  const { completed, step } = settings.data.onboarding;
  // Progress left on a retired step (Try it, UX-24) had nothing left to do.
  return completed || !isSetupStep(step) ? 'app' : 'onboarding';
}
