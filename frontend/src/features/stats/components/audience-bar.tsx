/**
 * Internal vs external as one part-to-whole bar: internal in the accent
 * ("yours"), external in the second speaker hue (validated against the
 * accent for colour-vision deficiencies), unknown in the muted fill, each
 * segment separated by a 2px gap. The legend under it names every part
 * with its count and share, so colour is never the only key.
 */
import { cn } from '@/ui/cn';

import { statsCopy } from '../copy';
import { percent } from '../format';
import type { AudienceSplit } from '../types';

const PARTS = [
  { key: 'internal', fill: 'bg-accent' },
  { key: 'external', fill: 'bg-speaker-2' },
  { key: 'unknown', fill: 'bg-bg-muted' },
] as const satisfies readonly { key: keyof AudienceSplit; fill: string }[];

/** Props of {@link AudienceBar}. */
export interface AudienceBarProps {
  audience: AudienceSplit;
}

/** The split, as a bar and a legend. */
export function AudienceBar({ audience }: AudienceBarProps) {
  const total = audience.internal + audience.external + audience.unknown;
  const shown = PARTS.filter((part) => audience[part.key] > 0);
  return (
    <div className="flex flex-col gap-3">
      <div className="flex h-2 gap-0.5" aria-hidden>
        {shown.map((part) => (
          <div
            key={part.key}
            className={cn('h-full rounded-xs first:rounded-l-full last:rounded-r-full', part.fill)}
            style={{ flexGrow: audience[part.key] }}
          />
        ))}
      </div>
      <ul className="flex flex-wrap gap-x-6 gap-y-1">
        {PARTS.filter((part) => part.key !== 'unknown' || audience.unknown > 0).map((part) => (
          <li key={part.key} className="flex items-center gap-2 text-sm text-fg-muted">
            <span aria-hidden className={cn('size-2 rounded-full', part.fill)} />
            <span>{statsCopy[part.key]}</span>
            <span className="font-semibold text-fg tabular-nums">{audience[part.key]}</span>
            <span className="text-fg-subtle tabular-nums">
              {String(percent(audience[part.key], total))}%
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}
