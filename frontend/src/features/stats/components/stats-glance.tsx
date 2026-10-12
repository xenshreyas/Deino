/**
 * The last 30 days in three numbers, for Home (UX-25: Stats left the
 * sidebar and lives on Home). It links to the full Stats page, with the
 * year in review. Nothing shows until there is a meeting to count.
 */
import { Link } from '@tanstack/react-router';
import type { ReactNode } from 'react';

import { statsCopy } from '../copy';
import { formatCount, formatHours } from '../format';
import { useStatsOverview } from '../queries';

/** One number, label over value, like the Stats page's tiles. */
function Figure({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex min-w-0 flex-col gap-1 border-border px-4 first:pl-0 [&:not(:first-child)]:border-l">
      <dt className="truncate text-xs text-fg-muted">{label}</dt>
      <dd className="text-lg font-semibold text-fg">{value}</dd>
    </div>
  );
}

/** Props of {@link StatsGlance}. */
export interface StatsGlanceProps {
  /** Renders the section's heading row, given its id and the "See all" link. */
  heading: (id: string, action: ReactNode) => ReactNode;
}

/** Meetings, hours and people over the last 30 days, or nothing. */
export function StatsGlance({ heading }: StatsGlanceProps) {
  const stats = useStatsOverview('last_30_days');
  if (stats.data === undefined || stats.data.meetings === 0) return null;
  const seeAll = (
    <Link to="/stats" className="text-xs text-fg-muted hover:text-fg">
      {statsCopy.seeAll}
    </Link>
  );
  return (
    <section aria-labelledby="home-stats" className="flex flex-col gap-2">
      {heading('home-stats', seeAll)}
      <dl className="grid grid-cols-3 px-3">
        <Figure label={statsCopy.meetings} value={formatCount(stats.data.meetings)} />
        <Figure label={statsCopy.hours} value={formatHours(stats.data.recordedSeconds)} />
        <Figure label={statsCopy.people} value={formatCount(stats.data.people)} />
      </dl>
    </section>
  );
}
