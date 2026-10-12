import type { ComponentType } from 'react';

import { isSetupStep, type SetupStep } from '../machine';
import { useOnboardingDispatch, useOnboardingProgress } from '../queries';
import type { StepProps } from './step-frame';
import { AiSetupStep } from './steps/ai-setup-step';
import { CalendarStep } from './steps/calendar-step';
import { PermissionsStep } from './steps/permissions-step';
import { ProfileStep } from './steps/profile-step';
import { WelcomeStep } from './steps/welcome-step';

/** The screen of each step; adding a step is one entry here and one in `STEPS`. */
const STEP_VIEWS: Record<SetupStep, ComponentType<StepProps>> = {
  welcome: WelcomeStep,
  permissions: PermissionsStep,
  ai_setup: AiSetupStep,
  calendar: CalendarStep,
  profile: ProfileStep,
};

/**
 * First-run setup (FND-11): the saved step's screen, remounted per step so
 * each starts fresh. The shell shows it until `onboarding.completed`.
 */
export function OnboardingFlow() {
  const progress = useOnboardingProgress();
  const dispatch = useOnboardingDispatch();
  // A retired step (saved by an older version) counts as finished: the gate shows the app.
  if (progress === undefined || !isSetupStep(progress.step)) {
    return <div className="h-screen w-screen bg-bg" />;
  }
  const View = STEP_VIEWS[progress.step];
  return <View key={progress.step} dispatch={dispatch} />;
}
