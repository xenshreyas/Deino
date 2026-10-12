/**
 * A run's status as a quiet badge, and its log: each step's result, and
 * the error of a failed one.
 */
import { cn } from '@/ui/cn';

import { runCopy } from '../copy';
import type { AutomationRun, AutomationRunStatus, RunStep } from '../types';

const TONE: Record<AutomationRunStatus, string> = {
  armed: 'bg-bg-muted text-fg-muted',
  pending: 'bg-bg-muted text-fg-muted',
  done: 'bg-accent-soft text-fg',
  failed: 'bg-danger-soft text-danger',
};

/** "Done", "Failed", "Queued" or "Armed"; "Not run" without a run. */
export function RunBadge({ run }: { run: AutomationRun | null | undefined }) {
  const label = run == null ? runCopy.never : runCopy.status[run.status];
  return (
    <span
      className={cn(
        'inline-flex h-5 items-center rounded-sm px-1.5 text-xs font-semibold whitespace-nowrap',
        run == null ? 'text-fg-subtle' : TONE[run.status],
      )}
      title={run == null ? undefined : runCopy.trigger[run.trigger.kind]}
    >
      {label}
    </span>
  );
}

/** Each step of the run with its result. */
export function RunLog({ run, label }: { run: AutomationRun; label: (step: RunStep) => string }) {
  if (run.steps.length === 0) return null;
  return (
    <ol className="grid gap-1 text-xs">
      {run.steps.map((step) => (
        <li key={step.position} className="flex flex-wrap gap-x-2">
          <span className="text-fg-muted">
            {String(step.position + 1)}. {label(step)}
          </span>
          <span className={step.status === 'failed' ? 'text-danger' : 'text-fg-subtle'}>
            {runCopy.step[step.status]}
          </span>
          {step.error === null ? null : <span className="text-danger">{step.error}</span>}
        </li>
      ))}
    </ol>
  );
}
