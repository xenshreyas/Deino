/**
 * The "Today" digest (LokalBot's, community.md §5): the user's items due
 * today, ready to tick, and how many are overdue. Self-contained, so Home
 * (ORG-02) can place it as is; the Action Items page shows it on top.
 */
import { Skeleton } from '@/ui/skeleton';

import { copy } from '../copy';
import { useFilterOptions, useToday, useUpdateActionItem } from '../queries';
import { ItemRow, type RowHandlers } from './item-row';

/** Props of {@link TodayDigest}. */
export interface TodayDigestProps {
  /** Shows the overdue items (the page filters to them). */
  onShowOverdue?: (() => void) | undefined;
  /** Render nothing when nothing is due or overdue (the page's choice). */
  hideWhenEmpty?: boolean;
  /** How many due-today items to list. */
  limit?: number;
}

/** The headline: "2 items due today · 1 overdue". */
function Headline({
  due,
  overdue,
  onShowOverdue,
}: {
  due: number;
  overdue: number;
  onShowOverdue?: (() => void) | undefined;
}) {
  return (
    <div className="flex h-8 items-center gap-2 px-3 text-xs">
      <span className="font-semibold text-fg-muted">{copy.today.label}</span>
      <span className="text-fg-subtle">{due === 0 ? copy.today.clear : copy.today.due(due)}</span>
      {overdue === 0 ? null : (
        <button
          type="button"
          onClick={onShowOverdue}
          disabled={onShowOverdue === undefined}
          title={copy.today.showOverdue}
          className="rounded-xs text-danger underline-offset-4 focus-visible:outline-2 focus-visible:outline-focus-ring enabled:hover:underline"
        >
          {copy.today.overdue(overdue)}
        </button>
      )}
    </div>
  );
}

/** The digest. */
export function TodayDigest({ onShowOverdue, hideWhenEmpty = false, limit = 3 }: TodayDigestProps) {
  const today = useToday();
  const options = useFilterOptions();
  const update = useUpdateActionItem();
  if (today.isPending) return <Skeleton className="mx-3 my-2 h-4 w-48" />;
  if (today.isError) return null;
  const { dueToday, overdue } = today.data;
  if (hideWhenEmpty && dueToday.length === 0 && overdue.length === 0) return null;
  const handlers: RowHandlers = {
    onEdit: (item, edit) => {
      update.mutate({ id: item.id, edit });
    },
  };
  const now = new Date();
  return (
    <section aria-label={copy.today.label} className="mb-6 rounded-lg bg-bg-subtle py-1">
      <Headline due={dueToday.length} overdue={overdue.length} onShowOverdue={onShowOverdue} />
      <ul className="flex flex-col">
        {dueToday.slice(0, limit).map((item) => (
          <ItemRow
            key={`${item.id}:${item.status}`}
            item={item}
            people={options.data?.people ?? []}
            now={now}
            handlers={handlers}
          />
        ))}
      </ul>
    </section>
  );
}
