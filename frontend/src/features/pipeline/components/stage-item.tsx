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

interface StageItemProps {
  stage: StageStatus;
  /** True while a re-run request is in flight; disables the retry button. */
  retrying: boolean;
  /** Asks to run `stage` again. */
  onRetry: (stage: StageId) => void;
}

/** One stage: its name, status, why it skipped or failed, and a retry on failure. */
export function StageItem({ stage, retrying, onRetry }: StageItemProps) {
  const label = stageLabel(stage.stage);
  const stopped = stage.status === 'failed' || stage.status === 'cancelled';
  return (
    <li className="flex min-w-0 items-center gap-1.5 text-xs">
      <span aria-hidden className={cn('size-1.5 shrink-0 rounded-full', DOT[stage.status])} />
      <span className="text-fg">{label}</span>
      <span className="text-fg-muted">{statusText(stage)}</span>
      {stage.message !== null && stage.status !== 'succeeded' && (
        <span className={cn('truncate', stopped ? 'text-danger' : 'text-fg-subtle')}>
          {stage.message}
        </span>
      )}
      {stopped && (
        <Button
          variant="ghost"
          size="sm"
          className="h-6 px-1.5 text-xs"
          disabled={retrying}
          aria-label={copy.retryStage(label)}
          onClick={() => {
            onRetry(stage.stage);
          }}
        >
          <RotateCcw {...ICON_PROPS} size={14} />
          {copy.retry}
        </Button>
      )}
    </li>
  );
}

/** "Retrying soon" for a pending stage that already failed once, else its status. */
function statusText(stage: StageStatus): string {
  if (stage.status === 'pending' && stage.attempts > 0) return copy.retrying;
  return statusLabels[stage.status];
}
