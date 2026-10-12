/**
 * Settings → Search index (AI-01): the local embedding model's download,
 * how many meetings are indexed by meaning, the backfill, and a keyword
 * reindex. Without the model, search simply stays keyword-only.
 */
import { type ReactNode, useState } from 'react';
import { toast } from 'sonner';

import { Alert, AlertDescription } from '@/ui/alert';
import { Button } from '@/ui/button';
import { CircleAlert, ICON_PROPS } from '@/ui/icons';
import { Skeleton } from '@/ui/skeleton';

import { searchCopy } from '../copy';
import {
  useIndexStatus,
  useModelDownloadProgress,
  useReindexAll,
  useStartBackfill,
} from '../queries';
import type { IndexStatus } from '../types';
import { ModelRow } from './index-model-row';

const copy = searchCopy.index;

/** The Search index page. */
export function IndexSettings() {
  const status = useIndexStatus();
  if (status.isError) {
    return (
      <Alert variant="destructive">
        <CircleAlert {...ICON_PROPS} />
        <AlertDescription>{copy.error}</AlertDescription>
      </Alert>
    );
  }
  return (
    <div className="flex flex-col gap-8">
      {status.data === undefined ? (
        <Skeleton className="h-16 w-full rounded-lg" />
      ) : (
        <Loaded status={status.data} />
      )}
    </div>
  );
}

function Loaded({ status }: { status: IndexStatus }) {
  const [downloaded, setDownloaded] = useState<number | null>(null);
  useModelDownloadProgress(setDownloaded);
  return (
    <>
      <Card label={copy.model}>
        {status.model === null ? null : (
          <ModelRow model={status.model} downloadedBytes={downloaded} />
        )}
      </Card>
      <Card label={copy.index}>
        <CoverageRow status={status} />
        <RebuildRow />
      </Card>
    </>
  );
}

function Card({ label, children }: { label: string; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-2">
      <h3 className="text-sm font-semibold text-fg">{label}</h3>
      <div
        role="group"
        aria-label={label}
        className="divide-y divide-border rounded-lg border border-border bg-surface"
      >
        {children}
      </div>
    </section>
  );
}

/** "412 of 430" meetings, and the backfill. */
function CoverageRow({ status }: { status: IndexStatus }) {
  const start = useStartBackfill();
  const { backfill } = status;
  const ready = status.model?.state.kind === 'ready';
  return (
    <div className="flex items-center justify-between gap-4 px-4 py-3">
      <div className="flex flex-col gap-0.5">
        <span className="text-sm text-fg">{copy.coverage}</span>
        <span className="text-xs text-fg-muted tabular-nums">{coverageLine(status)}</span>
      </div>
      <Button
        size="sm"
        variant="secondary"
        disabled={!ready || backfill.running || start.isPending}
        onClick={() => {
          start.mutate();
        }}
      >
        {copy.backfill}
      </Button>
    </div>
  );
}

/** "412 of 430 meetings", the backfill's progress, or its failures. */
function coverageLine({ backfill, indexedMeetings, totalMeetings }: IndexStatus): string {
  if (backfill.running) return copy.backfilling(backfill.done, backfill.total);
  const coverage = copy.coverageValue(indexedMeetings, totalMeetings);
  return backfill.failed > 0 ? `${coverage} · ${copy.failed(backfill.failed)}` : coverage;
}

function RebuildRow() {
  const reindex = useReindexAll();
  const rebuild = () => {
    reindex.mutate(undefined, {
      onError: (error) => toast.error(error.message),
    });
  };
  return (
    <div className="flex items-center justify-between gap-4 px-4 py-3">
      <div className="flex flex-col gap-0.5">
        <span className="text-sm text-fg">{copy.rebuild}</span>
        <span className="text-xs text-fg-muted">{copy.rebuildBody}</span>
      </div>
      <Button size="sm" variant="secondary" disabled={reindex.isPending} onClick={rebuild}>
        {copy.rebuildAction}
      </Button>
    </div>
  );
}
