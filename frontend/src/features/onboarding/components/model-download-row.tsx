import { useState } from 'react';

import { useMountEffect } from '@/hooks/useMountEffect';
import { Button } from '@/ui/button';
import { CircleCheck, ICON_PROPS } from '@/ui/icons';
import { Skeleton } from '@/ui/skeleton';

import { aiCopy } from '../copy';
import type { LocalModel } from '../hooks/use-local-models';

/**
 * Transcription's readiness as one plain line (UX-23): "Getting
 * transcription ready… 42%" over a hairline bar, then "Transcription is
 * ready". No model name. A missing model starts downloading as soon as the
 * line appears; a failed one waits for the user.
 */
export function TranscriptionStatus({ model }: { model: LocalModel }) {
  return (
    <div className="flex flex-col gap-2">
      <div className="flex min-h-8 items-center gap-4">
        <StatusText model={model} />
        {model.view.kind === 'missing' ? (
          <Button variant="secondary" size="sm" onClick={model.download}>
            {model.error === null ? aiCopy.download : aiCopy.retry}
          </Button>
        ) : null}
      </div>
      {model.view.kind === 'downloading' ? <ProgressBar percent={model.view.percent} /> : null}
      {model.error === null ? null : (
        <p role="alert" className="text-xs text-danger">
          {model.error}
        </p>
      )}
      <AutoStart model={model} />
    </div>
  );
}

/** The line's words for each download state. */
function StatusText({ model }: { model: LocalModel }) {
  const { view } = model;
  switch (view.kind) {
    case 'checking':
      return <Skeleton className="h-4 w-48" />;
    case 'ready':
      return (
        <span className="flex flex-1 items-center gap-1.5 text-sm text-fg">
          <CircleCheck {...ICON_PROPS} className="text-accent" />
          {aiCopy.transcriptionReady}
        </span>
      );
    case 'downloading':
      return (
        <span className="flex-1 text-sm text-fg tabular-nums" aria-live="polite">
          {aiCopy.gettingReady(view.percent)}
        </span>
      );
    case 'missing':
      return <span className="flex-1 text-sm text-fg-muted">{aiCopy.transcriptionMissing}</span>;
  }
}

/**
 * Starts a missing model once, when it is first seen missing. Setup also
 * mounts it for the search model, which downloads without a line (UX-21).
 */
export function AutoStart({ model }: { model: LocalModel }) {
  const [pending, setPending] = useState(true);
  if (!pending || model.view.kind !== 'missing') return null;
  return (
    <StartOnMount
      onStart={() => {
        setPending(false);
        model.download();
      }}
    />
  );
}

/** A thin determinate bar in the accent colour. */
function ProgressBar({ percent }: { percent: number }) {
  return (
    <div
      role="progressbar"
      aria-label={aiCopy.transcription}
      aria-valuenow={percent}
      aria-valuemin={0}
      aria-valuemax={100}
      className="h-0.5 overflow-hidden rounded-full bg-border"
    >
      <div
        className="h-full bg-accent transition-[width] duration-(--dur-base) ease-out"
        style={{ width: `${String(percent)}%` }}
      />
    </div>
  );
}

/** Calls `onStart` once, when it mounts. */
function StartOnMount({ onStart }: { onStart: () => void }) {
  useMountEffect(onStart);
  return null;
}
