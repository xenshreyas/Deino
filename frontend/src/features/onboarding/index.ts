/**
 * The onboarding feature (FND-11): first-run setup in six skippable steps
 * (welcome, permissions, AI setup, calendar, profile, a test recording),
 * resumable from Settings → Setup.
 *
 * | Export | Used by |
 * |---|---|
 * | {@link OnboardingFlow} (lazy: render it in `Suspense`), {@link useOnboardingGate} | the app shell, which shows the flow until setup is completed |
 * | {@link ResumeSetup} | Settings → General → Setup |
 * | {@link PermissionsSettings} | Settings → Recording → Permissions (UX-10) |
 * | {@link NotesProvider} | Settings → Notes → AI (UX-11) |
 *
 * Progress is `Settings.onboarding`, moved by the pure step machine
 * (`machine.ts`); permissions come from the `onboarding` plugin, and every
 * other step reuses the feature that owns it.
 */
export { onboardingApi } from './api';
export { LazyOnboardingFlow as OnboardingFlow } from './components/lazy-onboarding-flow';
export { NotesProvider, type NotesProviderProps } from './components/notes-provider';
export { PermissionsSettings } from './components/permissions-settings';
export { ResumeSetup } from './components/resume-setup';
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
