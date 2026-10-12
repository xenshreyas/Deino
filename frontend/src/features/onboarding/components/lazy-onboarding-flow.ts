import { lazy } from 'react';

/**
 * {@link import('./onboarding-flow').OnboardingFlow}, loaded on first render.
 *
 * Onboarding runs once per install, and its steps pull in half the app's
 * features (calendar, profile, recording, search, transcription, llm), so
 * the feature's index stays light: the shell, Settings and the mock IPC can
 * import it without loading (or cycling through) the steps. Render it
 * inside a `Suspense` boundary.
 */
export const LazyOnboardingFlow = lazy(async () => {
  const { OnboardingFlow } = await import('./onboarding-flow');
  return { default: OnboardingFlow };
});
