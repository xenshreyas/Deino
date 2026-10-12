import type { ReactNode } from 'react';

import { Button } from '@/ui/button';

import { flowCopy } from '../copy';
import { type OnboardingAction, STEPS, stepIndex } from '../machine';
import type { OnboardingStep } from '../types';
import { StepProgress } from './step-progress';

/** What every step component receives from the flow. */
export interface StepProps {
  /** Moves the flow (continue, skip, back, finish). */
  dispatch: (action: OnboardingAction) => void;
}

/** A step's main button; "Continue" when a step does not set one. */
export interface PrimaryAction {
  label: string;
  onClick: () => void;
  disabled?: boolean;
}

/** Props of {@link StepFrame}. */
export interface StepFrameProps extends StepProps {
  step: OnboardingStep;
  title: string;
  description: string;
  primary?: PrimaryAction;
  /** The quiet action beside the main button; "Skip" by default. */
  secondary?: PrimaryAction;
  children?: ReactNode;
}

/**
 * The page every onboarding step shares: a drag strip for the transparent
 * title bar, the progress, the step's title and body in a narrow column, and
 * a footer with Back, Skip and the main action. Calm by design (§3B): one
 * column, hairline footer, no cards.
 */
export function StepFrame({ step, title, description, children, ...actions }: StepFrameProps) {
  return (
    <div className="flex h-screen w-screen flex-col bg-bg text-fg">
      <div data-tauri-drag-region className="h-11 shrink-0" />
      <main className="min-h-0 flex-1 overflow-y-auto">
        <div className="mx-auto flex w-full max-w-[36rem] flex-col gap-8 px-6 pt-6 pb-12">
          <StepProgress step={step} />
          <header className="flex flex-col gap-2">
            <h1 className="text-2xl font-semibold tracking-tight text-fg">{title}</h1>
            <p className="text-sm leading-6 text-fg-muted">{description}</p>
          </header>
          {children}
        </div>
      </main>
      <StepFooter step={step} {...actions} />
    </div>
  );
}

type StepFooterProps = Pick<StepFrameProps, 'step' | 'dispatch' | 'primary' | 'secondary'>;

/** Back on the left; Skip (or the step's own quiet action) and the main action on the right. */
function StepFooter({ step, dispatch, primary, secondary }: StepFooterProps) {
  const main = primary ?? {
    label: flowCopy.continue,
    onClick: () => {
      dispatch({ type: 'continue' });
    },
  };
  const quiet = secondary ?? {
    label: flowCopy.skip,
    onClick: () => {
      dispatch({ type: 'skip' });
    },
  };
  const first = stepIndex(step) === 0;
  return (
    <footer className="shrink-0 border-t border-border">
      <div className="mx-auto flex w-full max-w-[36rem] items-center gap-2 px-6 py-4">
        {first ? null : (
          <Button
            variant="ghost"
            onClick={() => {
              dispatch({ type: 'back' });
            }}
          >
            {flowCopy.back}
          </Button>
        )}
        <span className="flex-1" />
        <Button variant="ghost" disabled={quiet.disabled} onClick={quiet.onClick}>
          {quiet.label}
        </Button>
        <Button size="lg" disabled={main.disabled} onClick={main.onClick}>
          {main.label}
        </Button>
      </div>
    </footer>
  );
}

/** The number of steps, for "Step 2 of 6". */
export const STEP_COUNT = STEPS.length;
