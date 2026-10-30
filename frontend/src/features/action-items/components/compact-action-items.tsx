/**
 * The meeting's action items in the meeting page's context rail: a quiet
 * list of the checkbox and title, with the assignee and due day on a second
 * line. Ticking one completes it everywhere, as on the Notes tab; editing
 * the rest stays there. The first few show, then "Show all"; nothing shows
 * until the meeting has items.
 */
import { useState } from 'react';

import type { MeetingId } from '@/features/meetings';
import { cn } from '@/ui/cn';
import { PanelSection } from '@/ui/panel-section';

import { copy } from '../copy';
import { dueLabel } from '../dates';
import { meetingFilter, openFirst } from '../notes-sync';
import { useActionItems, useUpdateActionItem } from '../queries';
import type { ActionItemView } from '../types';
import { ItemCheckbox } from './item-checkbox';

/** Rows shown before "Show all". */
const FIRST_ROWS = 5;

/** "Me · Tomorrow", "Ana", "Fri", or nothing. */
export function ownerAndDue(item: ActionItemView, now: Date): string {
  const owner = item.assigneeIsMe ? copy.assignee.me : item.assigneeName;
  const due = item.dueAt === null ? null : dueLabel(item.dueAt, now);
  return [owner, due].filter((part) => part !== null && part !== '').join(' · ');
}

/** One item: the checkbox, the title, and its owner and due day. */
function CompactRow({ item, now }: { item: ActionItemView; now: Date }) {
  const update = useUpdateActionItem();
  const done = item.status === 'done';
  const detail = ownerAndDue(item, now);
  return (
    <li className="flex items-start gap-2">
      <ItemCheckbox
        checked={done}
        label={done ? copy.row.reopen(item.title) : copy.row.complete(item.title)}
        onToggle={() => {
          update.mutate({ id: item.id, edit: { field: 'status', status: done ? 'open' : 'done' } });
        }}
        className="mt-0.5 size-3.5"
      />
      <span className="min-w-0 text-[13px]/[18px]">
        <span
          title={item.title}
          className={cn(
            'line-clamp-2 text-pretty',
            done ? 'text-fg-subtle line-through' : 'text-fg',
          )}
        >
          {item.title}
        </span>
        {detail === '' ? null : <span className="block text-xs text-fg-subtle">{detail}</span>}
      </span>
    </li>
  );
}

/** The meeting's items under their heading, or nothing. */
export function CompactActionItems({ meetingId }: { meetingId: MeetingId }) {
  const items = useActionItems(meetingFilter(meetingId));
  const [expanded, setExpanded] = useState(false);
  if (!items.isSuccess || items.data.length === 0) return null;
  const ordered = openFirst(items.data);
  const open = ordered.filter((item) => item.status === 'open').length;
  const shown = expanded ? ordered : ordered.slice(0, FIRST_ROWS);
  const now = new Date();
  return (
    <PanelSection
      title={copy.notes.heading}
      action={<span className="text-xs text-fg-subtle tabular-nums">{copy.notes.count(open)}</span>}
    >
      <ul className="flex flex-col gap-2">
        {shown.map((item) => (
          <CompactRow key={item.id} item={item} now={now} />
        ))}
      </ul>
      {ordered.length > FIRST_ROWS ? (
        <button
          type="button"
          onClick={() => {
            setExpanded(!expanded);
          }}
          className="self-start rounded-xs text-xs text-fg-muted underline-offset-4 hover:text-fg hover:underline focus-visible:outline-2 focus-visible:outline-focus-ring"
        >
          {expanded ? copy.notes.fewer : copy.notes.all(ordered.length)}
        </button>
      ) : null}
    </PanelSection>
  );
}
