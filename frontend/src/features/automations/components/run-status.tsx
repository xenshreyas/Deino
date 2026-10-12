/**
 * A run's status as a quiet badge, and its log: each step's result, and
 * the error of a failed one. Plus the short dates the run views share.
 */
import { cn } from '@/ui/cn';
import { Check, Circle, CircleAlert, type Icon, ICON_PROPS, X } from '@/ui/icons';

import { runCopy } from '../copy';
import type { AutomationRun, AutomationRunStatus, AutomationStepStatus, RunStep } from '../types';

const TONE: Record<AutomationRunStatus, string> = {
  armed: 'bg-bg-muted text-fg-muted',
  pending: 'bg-bg-muted text-fg-muted',
  done: 'bg-accent-soft text-fg',
  failed: 'bg-danger-soft text-danger',
};

const STEP_ICON: Record<AutomationStepStatus, { icon: Icon; tone: string }> = {
  pending: { icon: Circle, tone: 'text-fg-subtle' },
  done: { icon: Check, tone: 'text-fg-muted' },
  failed: { icon: CircleAlert, tone: 'text-danger' },
  skipped: { icon: X, tone: 'text-fg-subtle' },
};

/** "Done", "Failed", "Queued" or "Armed"; "Not run" without a run. */
export function RunBadge({ run }: { run: AutomationRun | null | undefined }) {
  const label = run == null ? runCopy.never : runCopy.status[run.status];
  return (
    <span
      className={cn(
        'inline-flex h-5 shrink-0 items-center rounded-sm px-1.5 text-xs font-semibold whitespace-nowrap',
        run == null ? 'text-fg-subtle' : TONE[run.status],
      )}
      title={run == null ? undefined : runCopy.trigger[run.trigger.kind]}
    >
      {label}
    </span>
  );
}

/** Each step of the run with its result, and a failed step's error. */
export function RunLog({ run, label }: { run: AutomationRun; label: (step: RunStep) => string }) {
  if (run.steps.length === 0) return null;
  return (
    <ol className="grid gap-1 text-xs">
      {run.steps.map((step) => {
        const { icon: StatusIcon, tone } = STEP_ICON[step.status];
        return (
          <li key={step.position} className="grid grid-cols-[14px_minmax(0,1fr)] gap-x-2">
            <StatusIcon {...ICON_PROPS} size={14} className={cn('mt-px', tone)} />
            <span className="text-fg-muted">
              {label(step)}
              <span className={step.status === 'failed' ? 'text-danger' : 'text-fg-subtle'}>
                {` · ${runCopy.step[step.status]}`}
              </span>
            </span>
            {step.error === null ? null : (
              <span className="col-start-2 text-danger">{step.error}</span>
            )}
          </li>
        );
      })}
    </ol>
  );
}

/** `Oct 8`, with the year when it is not this year. */
export function shortDate(iso: string): string {
  const date = new Date(iso);
  const sameYear = date.getFullYear() === new Date().getFullYear();
  return date.toLocaleDateString('en-US', {
    month: 'short',
    day: 'numeric',
    ...(sameYear ? {} : { year: 'numeric' }),
  });
}

/** `Oct 8, 2:05 PM`. */
export function shortDateTime(iso: string): string {
  const time = new Date(iso).toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' });
  return `${shortDate(iso)}, ${time}`;
}
