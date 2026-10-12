/**
 * The KPI row: four numbers, label over value, divided by hairlines rather
 * than boxed (no cards, ui.md §1 rule 4). Values use proportional figures.
 */
import { statsCopy } from '../copy';
import { formatCost, formatCount, formatHours } from '../format';
import type { StatsOverview } from '../types';

function Tile({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex min-w-0 flex-col gap-1 border-border px-4 first:pl-0 [&:not(:first-child)]:border-l">
      <dt className="truncate text-xs text-fg-muted">{label}</dt>
      <dd className="text-xl font-semibold text-fg">{value}</dd>
    </div>
  );
}

/** Props of {@link StatTiles}. */
export interface StatTilesProps {
  stats: StatsOverview;
}

/** Meetings, hours, people and the estimated LLM cost of the period. */
export function StatTiles({ stats }: StatTilesProps) {
  return (
    <dl className="grid grid-cols-2 gap-y-4 sm:grid-cols-4">
      <Tile label={statsCopy.meetings} value={formatCount(stats.meetings)} />
      <Tile label={statsCopy.hours} value={formatHours(stats.recordedSeconds)} />
      <Tile label={statsCopy.people} value={formatCount(stats.people)} />
      <Tile label={statsCopy.cost} value={formatCost(stats.llm.estimatedCostMicros)} />
    </dl>
  );
}
