/**
 * The results page's rows: a meeting with its best match, a moment with its
 * time in the recording, a person or company. Rows are buttons, so the
 * list reads and works from the keyboard.
 */
import { cn } from '@/ui/cn';
import { Skeleton } from '@/ui/skeleton';

import { searchCopy } from '../copy';
import { momentTime, shortDate } from '../format';
import { hitKey } from '../hit-target';
import type { Hit, HitKind } from '../types';
import { HighlightedText } from './highlighted-text';

/** Where in a meeting a hit matched, when not its title. */
const SOURCE: Partial<Record<HitKind, string>> = {
  segment: searchCopy.inTranscript,
  note: searchCopy.inNotes,
  summary: searchCopy.inSummary,
};

/** Props of {@link ResultList}. */
export interface ResultListProps {
  hits: readonly Hit[];
  /** Moments lead with their time; meetings with their title. */
  moments: boolean;
  onOpen: (hit: Hit) => void;
}

/** A list of hits, one row each. */
export function ResultList({ hits, moments, onOpen }: ResultListProps) {
  return (
    <ul className="flex flex-col gap-0.5 pt-3">
      {hits.map((hit) => (
        <li key={hitKey(hit)}>
          <button
            type="button"
            onClick={() => {
              onOpen(hit);
            }}
            className={cn(
              'flex w-full gap-3 rounded-md px-3 py-2.5 text-left',
              'transition-colors duration-(--dur-fast) ease-out hover:bg-bg-muted',
              'focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-focus-ring',
            )}
          >
            <ResultRow hit={hit} moment={moments} />
          </button>
        </li>
      ))}
    </ul>
  );
}

function ResultRow({ hit, moment }: { hit: Hit; moment: boolean }) {
  const isMoment = moment && hit.tsMs !== null;
  return (
    <>
      {isMoment ? (
        <span className="mt-0.5 h-5 shrink-0 rounded-sm bg-bg-muted px-1.5 text-xs leading-5 text-fg-muted tabular-nums">
          {momentTime(hit.tsMs ?? 0)}
        </span>
      ) : null}
      <span className="flex min-w-0 flex-1 flex-col gap-0.5">
        <span className="flex items-baseline gap-3">
          <HighlightedText
            snippet={hit.label}
            className="min-w-0 flex-1 truncate text-sm font-semibold text-fg"
          />
          <span className="shrink-0 text-xs text-fg-subtle tabular-nums">
            {shortDate(hit.date)}
          </span>
        </span>
        {hit.snippet.text === '' ? null : (
          <span className="line-clamp-2 text-sm leading-6 text-fg-muted">
            <HighlightedText snippet={hit.snippet} />
            {isMoment ? null : <Source kind={hit.kind} />}
          </span>
        )}
      </span>
    </>
  );
}

function Source({ kind }: { kind: HitKind }) {
  const source = SOURCE[kind];
  if (source === undefined) return null;
  return <span className="text-fg-subtle"> · {source}</span>;
}

/** Rows of grey bars while the first results load. */
export function ResultsSkeleton() {
  return (
    <div role="status" aria-label="Loading" className="flex flex-col gap-1 pt-3">
      {['w-11/12', 'w-3/4', 'w-5/6', 'w-3/5'].map((width) => (
        <div key={width} className="flex flex-col gap-2 px-3 py-3">
          <Skeleton className="h-2.5 w-1/3" />
          <Skeleton className={cn('h-2.5', width)} />
        </div>
      ))}
    </div>
  );
}
