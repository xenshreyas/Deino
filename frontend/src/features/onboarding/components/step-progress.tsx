import { cn } from '@/ui/cn';

import { flowCopy, stepNames } from '../copy';
import { STEPS, stepIndex } from '../machine';
import type { OnboardingStep } from '../types';

/**
 * Where the user is: "Step 2 of 6 · Permissions" over one hairline segment
 * per step, the steps so far in ink.
 */
export function StepProgress({ step }: { step: OnboardingStep }) {
  const current = stepIndex(step);
  return (
    <div className="flex flex-col gap-3">
      <p className="text-xs text-fg-muted">
        {flowCopy.progress(current + 1, STEPS.length)}
        <span aria-hidden="true"> · </span>
        {stepNames[step]}
      </p>
      <ol aria-label={flowCopy.progress(current + 1, STEPS.length)} className="flex gap-1">
        {STEPS.map((name, index) => (
          <li
            key={name}
            aria-current={index === current ? 'step' : undefined}
            className={cn(
              'h-0.5 flex-1 rounded-full transition-colors duration-(--dur-base)',
              index <= current ? 'bg-fg' : 'bg-border',
            )}
          >
            <span className="sr-only">{stepNames[name]}</span>
          </li>
        ))}
      </ol>
    </div>
  );
}
