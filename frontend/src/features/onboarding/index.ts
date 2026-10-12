/**
 * The onboarding feature (FND-11): first-run setup in five skippable steps
 * (welcome, permissions, AI setup, calendar, profile), resumable from
 * Settings → Setup, plus what moved out of setup (UX-22, UX-24).
 *
 * | Export | Used by |
 * |---|---|
 * | {@link OnboardingFlow} (lazy: render it in `Suspense`), {@link useOnboardingGate} | the app shell, which shows the flow until setup is completed |
 * | {@link ResumeSetup} | Settings → Setup |
 * | {@link TryDeino} | Home, while there are no notes: a test recording and the sample meeting |
 * | {@link PermissionNudge}, {@link askPermissionOnce} | optional permissions asked where they matter |
 *
 * Progress is `Settings.onboarding`, moved by the pure step machine
 * (`machine.ts`); permissions come from the `onboarding` plugin, and every
 * other step reuses the feature that owns it.
 */
export { onboardingApi } from './api';
export { askPermissionOnce, type ContextPermission } from './ask-in-context';
export { LazyOnboardingFlow as OnboardingFlow } from './components/lazy-onboarding-flow';
export { PermissionNudge } from './components/permission-nudge';
export { ResumeSetup } from './components/resume-setup';
export { TryDeino } from './components/try-deino';
export { type OnboardingGate, useOnboardingGate } from './hooks/use-onboarding-gate';
export { FRESH, type OnboardingAction, STEPS, transition } from './machine';
export { onboardingKeys, useOnboardingDispatch, useOnboardingProgress } from './queries';
export type {
  OnboardingSettings,
  OnboardingStep,
  PermissionKind,
  PermissionState,
  PermissionStatus,
  StepOutcome,
} from './types';
