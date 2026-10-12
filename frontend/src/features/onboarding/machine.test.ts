import { describe, expect, it } from 'vitest';

import {
  FRESH,
  firstUnfinished,
  type OnboardingAction,
  STEPS,
  transition,
  unfinishedSteps,
} from './machine';
import type { OnboardingSettings } from './types';

/** Applies `actions` in order, starting from `start`. */
function run(actions: OnboardingAction[], start: OnboardingSettings = FRESH): OnboardingSettings {
  return actions.reduce(transition, start);
}

const CONTINUE: OnboardingAction = { type: 'continue' };
const SKIP: OnboardingAction = { type: 'skip' };

describe('the onboarding step machine', () => {
  it('moves forward and records a finished step as done', () => {
    const progress = run([CONTINUE, CONTINUE]);

    expect(progress.step).toBe('ai_setup');
    expect(progress.outcomes).toEqual({ welcome: 'done', permissions: 'done' });
    expect(progress.completed).toBe(false);
  });

  it('lets every step be skipped and records it', () => {
    const progress = run([CONTINUE, SKIP, SKIP]);

    expect(progress.step).toBe('calendar');
    expect(progress.outcomes).toEqual({
      welcome: 'done',
      permissions: 'skipped',
      ai_setup: 'skipped',
    });
  });

  it('completes after the last step, finished or skipped', () => {
    expect(run(STEPS.map(() => CONTINUE)).completed).toBe(true);
    const skipped = run(STEPS.map(() => SKIP));

    expect(skipped.completed).toBe(true);
    expect(skipped.step).toBe('try_it');
    expect(Object.values(skipped.outcomes)).toEqual(STEPS.map(() => 'skipped'));
  });

  it('never downgrades a finished step to skipped when it is revisited', () => {
    const progress = run([CONTINUE, CONTINUE, { type: 'back' }, SKIP]);

    expect(progress.outcomes.permissions).toBe('done');
    expect(progress.step).toBe('ai_setup');
  });

  it('upgrades a skipped step once it is finished', () => {
    const progress = run([CONTINUE, SKIP, { type: 'back' }, CONTINUE]);

    expect(progress.outcomes.permissions).toBe('done');
  });

  it('goes back one step, and stays on the first', () => {
    expect(run([CONTINUE, CONTINUE, { type: 'back' }]).step).toBe('permissions');
    expect(run([{ type: 'back' }])).toEqual(FRESH);
  });

  it('opens any step without recording an outcome', () => {
    const progress = run([{ type: 'open', step: 'profile' }]);

    expect(progress.step).toBe('profile');
    expect(progress.outcomes).toEqual({});
  });

  it('finishes from anywhere, keeping the step for a later resume', () => {
    const progress = run([CONTINUE, { type: 'finish' }]);

    expect(progress.completed).toBe(true);
    expect(progress.step).toBe('permissions');
  });

  it('resumes at the first step that is not done', () => {
    const finished = run([CONTINUE, CONTINUE, SKIP, CONTINUE, { type: 'finish' }]);

    const resumed = transition(finished, { type: 'resume' });

    expect(resumed.completed).toBe(false);
    expect(resumed.step).toBe('ai_setup');
    expect(resumed.outcomes).toEqual(finished.outcomes);
  });

  it('resumes at the permissions step when everything is done', () => {
    const done = run(STEPS.map(() => CONTINUE));

    expect(transition(done, { type: 'resume' }).step).toBe('permissions');
  });

  it('never changes the progress it is given', () => {
    const start = run([CONTINUE]);
    const copy = structuredClone(start);

    transition(start, SKIP);

    expect(start).toEqual(copy);
  });
});

describe('what is left to do', () => {
  it('is every step but welcome on a fresh install', () => {
    expect(unfinishedSteps(FRESH)).toEqual(STEPS.slice(1));
    expect(firstUnfinished(FRESH)).toBe('permissions');
  });

  it('is the skipped and unreached steps', () => {
    const progress = run([CONTINUE, CONTINUE, SKIP, CONTINUE]);

    expect(unfinishedSteps(progress)).toEqual(['ai_setup', 'profile', 'try_it']);
  });
});
