/**
 * A line's text with its search hits marked: every hit in `--highlight`,
 * the current one in `--selection` with a 1px accent outline (ui.md
 * Transcript bubble, Search).
 */
import type { ReactNode } from 'react';

import { cn } from '@/ui/cn';

import type { SearchHit } from '../model/search-index';

/** Props of {@link HighlightedText}. */
export interface HighlightedTextProps {
  text: string;
  /** This line's hits, in order and not overlapping. */
  hits: readonly SearchHit[] | undefined;
  /** The ordinal of the current hit across the transcript. */
  current: number | undefined;
}

/** `text` split around `hits`, each hit in a `<mark>`. */
export function HighlightedText({ text, hits, current }: HighlightedTextProps) {
  if (hits === undefined || hits.length === 0) return text;
  const parts: ReactNode[] = [];
  let cursor = 0;
  for (const hit of hits) {
    parts.push(text.slice(cursor, hit.range.start));
    const isCurrent = hit.ordinal === current;
    parts.push(
      <mark
        key={hit.ordinal}
        data-current={isCurrent || undefined}
        aria-current={isCurrent || undefined}
        className={cn(
          'rounded-xs text-fg',
          isCurrent ? 'bg-selection outline outline-1 outline-accent' : 'bg-highlight',
        )}
      >
        {text.slice(hit.range.start, hit.range.end)}
      </mark>,
    );
    cursor = hit.range.end;
  }
  parts.push(text.slice(cursor));
  return <>{parts}</>;
}
