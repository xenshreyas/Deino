/**
 * The list's sticky groups: open items by due date (Overdue, Today, This
 * week, Later, No date), then done items. Order within a group is the
 * query's (soonest first, undated last).
 */
import { copy } from './copy';
import { DUE_GROUPS, type DueGroup, dueGroup } from './dates';
import type { ActionItemView } from './types';

/** A group of the list. */
export type ListGroupId = DueGroup | 'done';

/** One sticky header and its items. */
export interface ListGroup {
  id: ListGroupId;
  label: string;
  items: ActionItemView[];
}

/** `items` grouped for the list, seen from `now`; empty groups are left out. */
export function groupItems(items: readonly ActionItemView[], now: Date): ListGroup[] {
  const groups = new Map<ListGroupId, ActionItemView[]>();
  for (const item of items) {
    const id: ListGroupId = item.status === 'done' ? 'done' : dueGroup(item.dueAt, now);
    groups.set(id, [...(groups.get(id) ?? []), item]);
  }
  const order: ListGroupId[] = [...DUE_GROUPS, 'done'];
  return order.flatMap((id) => {
    const grouped = groups.get(id);
    return grouped === undefined ? [] : [{ id, label: copy.groups[id], items: grouped }];
  });
}
