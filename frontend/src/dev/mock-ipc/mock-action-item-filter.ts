/**
 * The Action Items filter in the mock: the same rules as
 * `deino_core::action_items::ItemQuery` and the `list_by_*.sql` queries,
 * over in-memory rows. Local days come from the browser's time zone, as the
 * app's come from the Mac's.
 */
import { addDays, nextMonday, parseISO, startOfDay } from 'date-fns';

import type {
  ActionItemFilter,
  ActionItemView,
  CompanyId,
  DueFilter,
  PersonId,
} from '@/lib/ipc/generated/action-items';

/** What a row is matched with besides its own fields. */
export interface RowFacts {
  item: ActionItemView;
  /** Companies of the meeting's attendees and of the assignee. */
  companies: readonly CompanyId[];
}

/** What the filter is resolved against. */
export interface FilterContext {
  me: PersonId | null;
  now: Date;
}

function scopeMatches(
  item: ActionItemView,
  filter: ActionItemFilter,
  me: PersonId | null,
): boolean {
  const assignee = item.assigneePersonId;
  switch (filter.assignee.kind) {
    case 'me':
      return me !== null && assignee === me;
    case 'person':
      return assignee === filter.assignee.personId;
    case 'others':
      return assignee !== null && assignee !== me;
    case 'unassigned':
      return assignee === null;
    case 'anyone':
      return true;
  }
}

function statusMatches(item: ActionItemView, filter: ActionItemFilter): boolean {
  if (filter.due === 'overdue' || filter.status === 'open') return item.status === 'open';
  return filter.status === 'any' || item.status === filter.status;
}

function dueMatches(dueAt: string | null, due: DueFilter | null, now: Date): boolean {
  if (due === null) return true;
  if (due === 'no_date') return dueAt === null;
  if (dueAt === null) return false;
  const at = parseISO(dueAt);
  const today = startOfDay(now);
  if (due === 'overdue') return at < today;
  const end = due === 'today' ? addDays(today, 1) : startOfDay(nextMonday(now));
  return at >= today && at < end;
}

function heldMatches(item: ActionItemView, filter: ActionItemFilter): boolean {
  const range = filter.dateRange;
  if (range === null) return true;
  const held = parseISO(item.meetingHeldAt ?? item.createdAt);
  if (range.from !== null && held < startOfDay(parseISO(range.from))) return false;
  return range.to === null || held < addDays(startOfDay(parseISO(range.to)), 1);
}

/** True when `row` passes every condition of `filter`. */
export function matches(row: RowFacts, filter: ActionItemFilter, context: FilterContext): boolean {
  const { item } = row;
  return (
    scopeMatches(item, filter, context.me) &&
    statusMatches(item, filter) &&
    (filter.meetingId === null || item.meetingId === filter.meetingId) &&
    // The fixtures carry no tags or folders: those filters match nothing.
    filter.tagId === null &&
    filter.folderId === null &&
    (filter.companyId === null || row.companies.includes(filter.companyId)) &&
    dueMatches(item.dueAt, filter.due, context.now) &&
    heldMatches(item, filter)
  );
}

/** The list order: dated items soonest first, then undated ones, then by ID. */
export function byDue(a: ActionItemView, b: ActionItemView): number {
  if (a.dueAt === b.dueAt) return a.id.localeCompare(b.id);
  if (a.dueAt === null) return 1;
  if (b.dueAt === null) return -1;
  return a.dueAt.localeCompare(b.dueAt);
}
