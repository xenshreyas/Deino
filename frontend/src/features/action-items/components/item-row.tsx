/**
 * One action item row (ui.md §3 List row, 40px): a select box on hover, the
 * completion checkbox, the title (edited in place), then the meeting link,
 * the assignee and the due day. Ticking draws the check and lets the row
 * linger a moment before the list moves on, so the completion is seen.
 */
import { type MouseEvent, useState } from 'react';

import { cn } from '@/ui/cn';

import { copy } from '../copy';
import type { SelectClick } from '../selection';
import type { ActionItemEdit, ActionItemStatus, ActionItemView, PersonOption } from '../types';
import { AssigneeChip, DueChip, MeetingChip, PriorityMark } from './item-chips';
import { ItemCheckbox } from './item-checkbox';
import { ItemTitle } from './item-title';

/** How long a ticked row stays before it is saved and leaves an "open" list. */
export const LINGER_MS = 450;

/** What a row can do. */
export interface RowHandlers {
  onEdit: (item: ActionItemView, edit: ActionItemEdit) => void;
  /** Absent where rows cannot be selected (the Notes tab). */
  onSelect?: ((click: SelectClick) => void) | undefined;
}

/** Props of {@link ItemRow}. */
export interface ItemRowProps {
  item: ActionItemView;
  people: readonly PersonOption[];
  now: Date;
  handlers: RowHandlers;
  selected?: boolean;
  /** Show the meeting link (not on the meeting's own page). */
  showMeeting?: boolean;
}

/** The select box at the row's start, shown on hover or while selected. */
function SelectBox({
  item,
  selected,
  onSelect,
}: {
  item: ActionItemView;
  selected: boolean;
  onSelect: (click: SelectClick) => void;
}) {
  return (
    <button
      type="button"
      aria-pressed={selected}
      aria-label={copy.row.select(item.title)}
      onClick={(event) => {
        event.stopPropagation();
        onSelect({ id: item.id, range: event.shiftKey });
      }}
      className={cn(
        'absolute top-1/2 -left-6 size-3.5 -translate-y-1/2 rounded-xs border transition-opacity duration-(--dur-fast)',
        'focus-visible:opacity-100 focus-visible:outline-2 focus-visible:outline-focus-ring',
        selected
          ? 'border-fg bg-fg opacity-100'
          : 'border-border-strong opacity-0 group-hover:opacity-100',
      )}
    />
  );
}

/** The completion state, ticked at once and saved after {@link LINGER_MS}. */
function useCompletion(item: ActionItemView, onEdit: RowHandlers['onEdit']) {
  const [ticked, setTicked] = useState<boolean | null>(null);
  const done = ticked ?? item.status === 'done';
  const toggle = () => {
    const next = !done;
    const status: ActionItemStatus = next ? 'done' : 'open';
    setTicked(next);
    window.setTimeout(
      () => {
        onEdit(item, { field: 'status', status });
      },
      next ? LINGER_MS : 0,
    );
  };
  return { done, toggle };
}

/** The chips after the title: priority, meeting, assignee, due day. */
function Trailing({
  item,
  people,
  now,
  showMeeting,
  edit,
}: Pick<ItemRowProps, 'item' | 'people' | 'now'> & {
  showMeeting: boolean;
  edit: (change: ActionItemEdit) => void;
}) {
  return (
    <div className="ml-auto flex min-w-0 shrink-0 items-center gap-1">
      <PriorityMark item={item} />
      {showMeeting ? <MeetingChip item={item} /> : null}
      <AssigneeChip
        item={item}
        people={people}
        onChange={(personId) => {
          edit({ field: 'assignee', personId });
        }}
      />
      <DueChip
        item={item}
        now={now}
        onChange={(date) => {
          edit({ field: 'due', date });
        }}
      />
    </div>
  );
}

/** One row. Remount it (key on the status) when the stored status changes. */
export function ItemRow(props: ItemRowProps) {
  const { item, handlers, selected = false, showMeeting = true } = props;
  const { done, toggle } = useCompletion(item, handlers.onEdit);
  const { onSelect } = handlers;
  const edit = (change: ActionItemEdit) => {
    handlers.onEdit(item, change);
  };
  const rowClick = (event: MouseEvent<HTMLLIElement>) => {
    onSelect?.({ id: item.id, range: event.shiftKey });
  };
  return (
    <li
      data-selected={selected}
      onClick={rowClick}
      className={cn(
        'group relative flex h-10 items-center gap-3 rounded-md px-3 transition-colors duration-(--dur-fast) ease-out',
        selected ? 'bg-accent-soft' : 'hover:bg-bg-muted',
      )}
    >
      {onSelect === undefined ? null : (
        <SelectBox item={item} selected={selected} onSelect={onSelect} />
      )}
      <ItemCheckbox
        checked={done}
        onToggle={toggle}
        label={(done ? copy.row.reopen : copy.row.complete)(item.title)}
      />
      <ItemTitle
        title={item.title}
        done={done}
        onSave={(title) => {
          edit({ field: 'title', title });
        }}
      />
      <Trailing {...props} showMeeting={showMeeting} edit={edit} />
    </li>
  );
}
