import { Button } from '@/ui/button';

import { importCopy, phaseCopy, sourceCopy } from '../copy';
import { useCancelImport } from '../queries';
import type { ImportProgress } from '../types';

/** True while the job runs. */
export function isRunning(progress: ImportProgress | undefined): boolean {
  return progress?.phase === 'listing' || progress?.phase === 'importing';
}

/** The running or last import: its bar, counts and Stop (container). */
export function ImportStatus({ progress }: { progress: ImportProgress }) {
  const cancel = useCancelImport();
  const running = isRunning(progress);
  const done = progress.imported + progress.failed;
  const percent = progress.total === 0 ? 0 : Math.round((done / progress.total) * 100);
  const source = progress.source === null ? '' : `${sourceCopy[progress.source].name} · `;
  return (
    <div className="flex flex-col gap-2 rounded-lg border border-border bg-surface px-4 py-3">
      <div className="flex items-center justify-between gap-4">
        <p role="status" className="text-sm text-fg">
          {source}
          {phaseCopy[progress.phase]}
          {progress.phase === 'importing' ? (
            <span className="ml-2 text-fg-muted tabular-nums">
              {importCopy.progress(done, progress.total)}
            </span>
          ) : null}
        </p>
        {running ? (
          <Button
            variant="ghost"
            size="sm"
            disabled={cancel.isPending}
            onClick={() => {
              cancel.mutate();
            }}
          >
            {importCopy.cancel}
          </Button>
        ) : null}
      </div>
      {running ? <ProgressBar percent={percent} /> : null}
      <StatusDetail progress={progress} running={running} />
    </div>
  );
}

function ProgressBar({ percent }: { percent: number }) {
  return (
    <div className="h-1 overflow-hidden rounded-full bg-bg-muted">
      <div
        role="progressbar"
        aria-valuenow={percent}
        aria-valuemin={0}
        aria-valuemax={100}
        className="h-full origin-left bg-accent transition-transform duration-(--dur-slow) ease-out"
        style={{ transform: `scaleX(${String(percent / 100)})` }}
      />
    </div>
  );
}

function StatusDetail({ progress, running }: { progress: ImportProgress; running: boolean }) {
  if (progress.error !== null) {
    return (
      <p role="alert" className="text-xs text-danger">
        {progress.error}
      </p>
    );
  }
  if (running && progress.phase === 'listing') return null;
  return (
    <p className="text-xs text-fg-muted">
      {importCopy.summary(progress.imported, progress.skipped, progress.failed)}
    </p>
  );
}
