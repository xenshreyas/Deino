/**
 * The meeting's action items at the top of its Notes tab (ui.md §4.2:
 * "Action items", then one row per item with its assignee and due day).
 * Ticking one completes it everywhere. Long lists show their first few with
 * "Show all", so the notes stay in view; nothing shows until the meeting has
 * items.
 */
import { useState } from 'react';

import type { MeetingId } from '@/features/meetings';

import { copy } from '../copy';
import { meetingFilter, openFirst } from '../notes-sync';
import { useActionItems, useFilterOptions, useUpdateActionItem } from '../queries';
import type { ActionItemView } from '../types';
import { ItemRow, type RowHandlers } from './item-row';

/** Rows shown before "Show all". */
const FIRST_ROWS = 5;

/** Props of {@link MeetingActionItems}. */
export interface MeetingActionItemsProps {
  meetingId: MeetingId;
}

/** The rows, the first few or all. */
function Rows({ items, expanded }: { items: readonly ActionItemView[]; expanded: boolean }) {
  const people = useFilterOptions().data?.people ?? [];
  const update = useUpdateActionItem();
  const handlers: RowHandlers = {
    onEdit: (item, edit) => {
      update.mutate({ id: item.id, edit });
    },
  };
  const now = new Date();
  const shown = expanded ? items : items.slice(0, FIRST_ROWS);
  return (
    <ul className="-mx-3 mt-1 flex flex-col">
      {shown.map((item) => (
        <ItemRow
          key={`${item.id}:${item.status}`}
          item={item}
          people={people}
          now={now}
          handlers={handlers}
          showMeeting={false}
        />
      ))}
    </ul>
  );
}

/** The heading and rows, or nothing. */
export function MeetingActionItems({ meetingId }: MeetingActionItemsProps) {
  const items = useActionItems(meetingFilter(meetingId));
  const [expanded, setExpanded] = useState(false);
  if (!items.isSuccess || items.data.length === 0) return null;
  const ordered = openFirst(items.data);
  const open = ordered.filter((item) => item.status === 'open').length;
  const hidden = ordered.length - FIRST_ROWS;
  return (
    <section aria-labelledby="meeting-action-items" className="mt-4 mb-2 shrink-0">
      <h2
        id="meeting-action-items"
        className="flex items-baseline gap-2 text-base font-semibold text-fg"
      >
        {copy.notes.heading}
        <span className="text-xs font-normal text-fg-subtle tabular-nums">
          {copy.notes.count(open)}
        </span>
      </h2>
      <Rows items={ordered} expanded={expanded} />
      {hidden > 0 ? (
        <button
          type="button"
          onClick={() => {
            setExpanded(!expanded);
          }}
          className="mt-1 rounded-xs text-xs text-fg-muted underline-offset-4 hover:text-fg hover:underline focus-visible:outline-2 focus-visible:outline-focus-ring"
        >
          {expanded ? copy.notes.fewer : copy.notes.all(ordered.length)}
        </button>
      ) : null}
    </section>
  );
}
