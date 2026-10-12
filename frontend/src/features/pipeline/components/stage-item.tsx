import { Button } from '@/ui/button';
import { cn } from '@/ui/cn';
import { ICON_PROPS, RotateCcw } from '@/ui/icons';

import { copy, stageLabel, statusLabels } from '../copy';
import type { JobStatus, StageId, StageStatus } from '../types';

/** The dot beside each stage, by status; design tokens only (§3B). */
const DOT: Record<JobStatus, string> = {
  pending: 'bg-fg-subtle',
  running: 'bg-accent motion-safe:animate-pulse',
  succeeded: 'bg-success',
  skipped: 'bg-fg-subtle',
  failed: 'bg-danger',
  cancelled: 'bg-danger',
};

/** Whether a stage stopped short and can be retried. */
export function isStopped(stage: StageStatus): boolean {
  return stage.status === 'failed' || stage.status === 'cancelled';
}

interface StageItemProps {
  stage: StageStatus;
  /** True while a re-run request is in flight; disables the retry button. */
  retrying: boolean;
  /** Asks to run `stage` again. */
  onRetry: (stage: StageId) => void;
}

/**
 * One row of Processing details: the stage's name and status, why it
 * skipped or failed, and a retry on failure.
 */
export function StageItem({ stage, retrying, onRetry }: StageItemProps) {
  const label = stageLabel(stage.stage);
  const stopped = isStopped(stage);
  return (
    <li className="grid grid-cols-[auto_1fr_auto] items-start gap-x-2.5 py-1.5 text-sm">
      <span aria-hidden className={cn('mt-2 size-1.5 rounded-full', DOT[stage.status])} />
      <div className="flex min-w-0 flex-col">
        <p className="flex items-baseline gap-2">
          <span className="text-fg">{label}</span>
          <span className="text-xs text-fg-muted">{statusText(stage)}</span>
        </p>
        {stage.message !== null && stage.status !== 'succeeded' && (
          <p className={cn('text-xs text-pretty', stopped ? 'text-danger' : 'text-fg-subtle')}>
            {stage.message}
          </p>
        )}
      </div>
      {stopped ? (
        <RetryButton
          label={label}
          disabled={retrying}
          onRetry={() => {
            onRetry(stage.stage);
          }}
        />
      ) : null}
    </li>
  );
}

/** "Retry", named for the stage it re-runs. */
export function RetryButton({
  label,
  disabled,
  onRetry,
}: {
  label: string;
  disabled: boolean;
  onRetry: () => void;
}) {
  return (
    <Button
      variant="ghost"
      size="sm"
      className="h-6 px-1.5 text-xs"
      disabled={disabled}
      aria-label={copy.retryStage(label)}
      onClick={onRetry}
    >
      <RotateCcw {...ICON_PROPS} size={14} />
      {copy.retry}
    </Button>
  );
}

/** "Retrying soon" for a pending stage that already failed once, else its status. */
function statusText(stage: StageStatus): string {
  if (stage.status === 'pending' && stage.attempts > 0) return copy.retrying;
  return statusLabels[stage.status];
}
