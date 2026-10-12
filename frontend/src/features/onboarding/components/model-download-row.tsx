import { useState } from 'react';

import { useMountEffect } from '@/hooks/useMountEffect';
import { Button } from '@/ui/button';
import { CircleCheck, ICON_PROPS } from '@/ui/icons';
import { Skeleton } from '@/ui/skeleton';

import { aiCopy } from '../copy';
import { sizeLabel } from '../download-view';
import type { LocalModel } from '../hooks/use-local-models';

/** Props of {@link ModelDownloadRow}. */
export interface ModelDownloadRowProps {
  name: string;
  why: string;
  model: LocalModel;
}

/**
 * One local model: its name and size, then its download (a hairline
 * progress bar), "Ready", or a Download button. A missing model starts
 * downloading as soon as the row appears; a failed one waits for the user.
 */
export function ModelDownloadRow({ name, why, model }: ModelDownloadRowProps) {
  const [autoStart, setAutoStart] = useState(true);
  const size = model.sizeBytes === null ? null : sizeLabel(model.sizeBytes);
  return (
    <li className="flex flex-col gap-2 py-4">
      <div className="flex items-center gap-4">
        <div className="flex min-w-0 flex-1 flex-col gap-0.5">
          <span className="text-sm font-semibold text-fg">
            {name}
            {size === null ? null : <span className="font-normal text-fg-subtle"> · {size}</span>}
          </span>
          <span className="text-sm text-fg-muted">{why}</span>
        </div>
        <DownloadControl model={model} />
      </div>
      {model.view.kind === 'downloading' ? <ProgressBar percent={model.view.percent} /> : null}
      {model.error === null ? null : (
        <p role="alert" className="text-xs text-danger">
          {model.error}
        </p>
      )}
      {autoStart && model.view.kind === 'missing' ? (
        <StartOnMount
          onStart={() => {
            setAutoStart(false);
            model.download();
          }}
        />
      ) : null}
    </li>
  );
}

/** The right side of the row for each download state. */
function DownloadControl({ model }: { model: LocalModel }) {
  const { view } = model;
  switch (view.kind) {
    case 'checking':
      return <Skeleton className="h-4 w-16" />;
    case 'ready':
      return (
        <span className="flex items-center gap-1.5 text-sm text-fg-muted">
          <CircleCheck {...ICON_PROPS} className="text-accent" />
          {aiCopy.ready}
        </span>
      );
    case 'downloading':
      return (
        <span className="text-sm text-fg-muted tabular-nums">
          {aiCopy.downloading(view.percent)}
        </span>
      );
    case 'missing':
      return (
        <Button variant="secondary" size="sm" onClick={model.download}>
          {model.error === null ? aiCopy.download : aiCopy.retry}
        </Button>
      );
  }
}

/** A thin determinate bar in the accent colour. */
function ProgressBar({ percent }: { percent: number }) {
  return (
    <div
      role="progressbar"
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
