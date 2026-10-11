/**
 * The notes adapter: a meeting's action items as todo blocks, for any view
 * of the meeting's notes. The Notes tab's list uses it today; NOT-02's
 * enhanced notes render the same `ActionItemTodo`s as TipTap task items and
 * call `setChecked` when one is ticked, so ticking either one updates the
 * item, and `DbChanged` refreshes every other view of it (the Action Items
 * page, the digest, another window).
 */
import { useMemo } from 'react';

import type { MeetingId } from '@/features/meetings';

import { useActionItems, useUpdateActionItem } from './queries';
import type { ActionItemFilter, ActionItemId, ActionItemView } from './types';

/** One action item as a todo block. */
export interface ActionItemTodo {
  id: ActionItemId;
  /** The title, the block's text. */
  text: string;
  checked: boolean;
  /** The assignee's name ("Me" is the caller's to render), if any. */
  assignee: string | null;
  /** The due instant (ISO), if any; the block shows its local day. */
  dueAt: string | null;
}

/** Every item of a meeting, whoever owns it and whatever its status. */
export function meetingFilter(meetingId: MeetingId): ActionItemFilter {
  return {
    assignee: { kind: 'anyone' },
    status: 'any',
    meetingId,
    tagId: null,
    companyId: null,
    folderId: null,
    due: null,
    dateRange: null,
  };
}

/** `item` as a todo block. */
export function todoOf(item: ActionItemView): ActionItemTodo {
  return {
    id: item.id,
    text: item.title,
    checked: item.status === 'done',
    assignee: item.assigneeName,
    dueAt: item.dueAt,
  };
}

/** Open items first, then done ones, each in the list's order. */
export function openFirst(items: readonly ActionItemView[]): ActionItemView[] {
  return [
    ...items.filter((item) => item.status === 'open'),
    ...items.filter((item) => item.status === 'done'),
  ];
}

/** A meeting's items as todos, and the tick that updates one. */
export function useActionItemTodos(meetingId: MeetingId) {
  const items = useActionItems(meetingFilter(meetingId));
  const update = useUpdateActionItem();
  const todos = useMemo(() => openFirst(items.data ?? []).map(todoOf), [items.data]);
  const setChecked = (id: ActionItemId, checked: boolean) => {
    update.mutate({ id, edit: { field: 'status', status: checked ? 'done' : 'open' } });
  };
  return { todos, setChecked, isPending: items.isPending };
}
