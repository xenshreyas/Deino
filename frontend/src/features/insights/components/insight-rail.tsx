/**
 * The Insights page's list of insights: a quiet rail of names with how many
 * meetings each has answered, the selected one in ink on a muted fill.
 */
import { cn } from '@/ui/cn';

import { insightsCopy } from '../copy';
import type { InsightId, InsightSummary } from '../types';

/** Props of {@link InsightRail}. */
export interface InsightRailProps {
  insights: readonly InsightSummary[];
  selected: InsightId;
  onSelect: (id: InsightId) => void;
}

/** One button per insight. */
export function InsightRail({ insights, selected, onSelect }: InsightRailProps) {
  return (
    <nav aria-label={insightsCopy.list.label}>
      <ul className="grid gap-0.5">
        {insights.map(({ def, progress }) => {
          const active = def.id === selected;
          return (
            <li key={def.id}>
              <button
                type="button"
                aria-current={active ? 'page' : undefined}
                onClick={() => {
                  onSelect(def.id);
                }}
                className={cn(
                  'flex h-[30px] w-full items-center gap-2 rounded-md px-2 text-left text-sm',
                  'transition-colors duration-(--dur-fast) ease-out hover:bg-bg-muted hover:text-fg',
                  'focus-visible:outline-2 focus-visible:outline-focus-ring',
                  active ? 'bg-bg-muted font-semibold text-fg' : 'text-fg-muted',
                )}
              >
                <span className="min-w-0 flex-1 truncate">{def.name}</span>
                <span
                  className="shrink-0 text-xs font-normal text-fg-subtle tabular-nums"
                  title={insightsCopy.list.answers(progress.done)}
                >
                  {progress.done}
                </span>
              </button>
            </li>
          );
        })}
      </ul>
    </nav>
  );
}
