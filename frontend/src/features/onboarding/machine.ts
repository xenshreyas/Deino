/**
 * The onboarding step machine (FND-11): six steps in a fixed order, each
 * finished or skipped, with the progress saved in `Settings.onboarding`.
 *
 * {@link transition} is pure, so every rule is unit-tested here and the
 * components only dispatch {@link OnboardingAction}s. Nothing is ever
 * forced: every step can be skipped, and Settings can resume a skipped one.
 */
import type { OnboardingSettings, OnboardingStep, StepOutcome } from './types';

/** Every step, in the order the flow shows them. */
export const STEPS = [
  'welcome',
  'permissions',
  'ai_setup',
  'calendar',
  'profile',
  'try_it',
] as const satisfies readonly OnboardingStep[];

/** Fails to compile if Rust adds a step that {@link STEPS} does not list. */
type MissingStep = Exclude<OnboardingStep, (typeof STEPS)[number]>;
const everyStepListed: MissingStep extends never ? true : never = true;
void everyStepListed;

/** Something the user did in the flow. */
export type OnboardingAction =
  /** Finished the step on screen. */
  | { type: 'continue' }
  /** Left the step on screen without finishing it. */
  | { type: 'skip' }
  /** Went back one step. */
  | { type: 'back' }
  /** Jumped to a step. */
  | { type: 'open'; step: OnboardingStep }
  /** Left setup for the app, skipping whatever is left. */
  | { type: 'finish' }
  /** Reopened setup from Settings. */
  | { type: 'resume' };

/** Progress before anything happened: a fresh install. */
export const FRESH: OnboardingSettings = { completed: false, step: 'welcome', outcomes: {} };

/** The step's position in {@link STEPS}. */
export function stepIndex(step: OnboardingStep): number {
  return STEPS.indexOf(step);
}

/** The step after `step`, or `null` after the last one. */
function nextStep(step: OnboardingStep): OnboardingStep | null {
  return STEPS[stepIndex(step) + 1] ?? null;
}

/** The step before `step`, or `step` itself when it is the first. */
function previousStep(step: OnboardingStep): OnboardingStep {
  return STEPS[stepIndex(step) - 1] ?? step;
}

/** Records how the current step was left, then moves on (or completes). */
function leave(progress: OnboardingSettings, outcome: StepOutcome): OnboardingSettings {
  const { step } = progress;
  const kept = progress.outcomes[step] === 'done' ? 'done' : outcome;
  const outcomes = { ...progress.outcomes, [step]: kept };
  const next = nextStep(step);
  return next === null
    ? { ...progress, completed: true, outcomes }
    : { ...progress, step: next, outcomes };
}

/**
 * The first step that still has something to do, ignoring the welcome
 * screen; the permissions step when every step is done.
 */
export function firstUnfinished(progress: OnboardingSettings): OnboardingStep {
  const pending = STEPS.find((step) => step !== 'welcome' && progress.outcomes[step] !== 'done');
  return pending ?? 'permissions';
}

/** The steps the user skipped or never reached, welcome excluded. */
export function unfinishedSteps(progress: OnboardingSettings): OnboardingStep[] {
  return STEPS.filter((step) => step !== 'welcome' && progress.outcomes[step] !== 'done');
}

/** The progress after `action`; the input is never changed. */
export function transition(
  progress: OnboardingSettings,
  action: OnboardingAction,
): OnboardingSettings {
  switch (action.type) {
    case 'continue':
      return leave(progress, 'done');
    case 'skip':
      return leave(progress, 'skipped');
    case 'back':
      return { ...progress, step: previousStep(progress.step) };
    case 'open':
      return { ...progress, step: action.step };
    case 'finish':
      return { ...progress, completed: true };
    case 'resume':
      return { ...progress, completed: false, step: firstUnfinished(progress) };
  }
}
