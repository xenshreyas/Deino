/**
 * The trailing chips of an item row: the meeting it came from (a link to
 * the moment it was said), the assignee and the due day, each opening its
 * picker. Unset chips stay out of sight until the row is hovered.
 */
import { Link } from '@tanstack/react-router';
import { forwardRef, type ButtonHTMLAttributes } from 'react';

import { cn } from '@/ui/cn';
import { ArrowUpRight, Calendar, Flag, ICON_PROPS, UserRound } from '@/ui/icons';

import { copy } from '../copy';
import { dueDay, dueGroup, dueLabel } from '../dates';
import type { ActionItemView, PersonOption } from '../types';
import { AssigneePicker, Avatar } from './assignee-picker';
import { DuePicker } from './due-picker';

const CHIP =
  'inline-flex h-6 shrink-0 items-center gap-1.5 rounded-full px-2 text-xs whitespace-nowrap text-fg-muted transition-colors duration-(--dur-fast) ease-out hover:bg-bg-muted hover:text-fg focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-focus-ring data-[state=open]:bg-bg-muted';
/** Chips with nothing to show yet: visible on hover, focus or while open. */
const RESTING =
  'opacity-0 group-hover:opacity-100 focus-visible:opacity-100 data-[state=open]:opacity-100';

/** A chip button that a picker can anchor to. */
const ChipButton = forwardRef<HTMLButtonElement, ButtonHTMLAttributes<HTMLButtonElement>>(
  ({ className, ...props }, ref) => (
    <button ref={ref} type="button" className={cn(CHIP, className)} {...props} />
  ),
);
ChipButton.displayName = 'ChipButton';

/** The meeting's title, linking to the moment the item was said. */
export function MeetingChip({ item }: { item: ActionItemView }) {
  if (item.meetingId === null) return null;
  const start = item.sourceStart ?? undefined;
  return (
    <Link
      to="/meetings/$meetingId"
      params={{ meetingId: item.meetingId }}
      search={{ tab: start === undefined ? 'notes' : 'transcript', t: start }}
      title={copy.row.openMoment}
      onClick={(event) => {
        event.stopPropagation();
      }}
      className={cn(CHIP, 'max-w-48 gap-1')}
    >
      <span className="truncate">{item.meetingTitle}</span>
      <ArrowUpRight {...ICON_PROPS} size={12} className="shrink-0" />
    </Link>
  );
}

/** The assignee, opening the people picker. */
export function AssigneeChip({
  item,
  people,
  onChange,
}: {
  item: ActionItemView;
  people: readonly PersonOption[];
  onChange: (person: ActionItemView['assigneePersonId']) => void;
}) {
  const name = item.assigneeIsMe ? copy.assignee.me : item.assigneeName;
  return (
    <AssigneePicker people={people} value={item.assigneePersonId} onChange={onChange}>
      <ChipButton
        aria-label={copy.assignee.set(item.title)}
        className={name === null ? RESTING : undefined}
      >
        {name === null ? (
          <UserRound {...ICON_PROPS} size={14} />
        ) : (
          <>
            <Avatar name={item.assigneeName ?? name} />
            <span className="max-w-28 truncate">{name}</span>
          </>
        )}
      </ChipButton>
    </AssigneePicker>
  );
}

/** The due day (red once overdue), opening the date picker. */
export function DueChip({
  item,
  now,
  onChange,
}: {
  item: ActionItemView;
  now: Date;
  onChange: (day: string | null) => void;
}) {
  const overdue = item.status === 'open' && dueGroup(item.dueAt, now) === 'overdue';
  return (
    <DuePicker value={dueDay(item.dueAt)} onChange={onChange}>
      <ChipButton
        aria-label={copy.due.set(item.title)}
        className={cn(
          'tabular-nums',
          item.dueAt === null && RESTING,
          overdue && 'text-danger hover:text-danger',
        )}
      >
        <Calendar {...ICON_PROPS} size={14} />
        {item.dueAt === null ? null : dueLabel(item.dueAt, now)}
      </ChipButton>
    </DuePicker>
  );
}

/** A small flag for urgent items. */
export function PriorityMark({ item }: { item: ActionItemView }) {
  if (item.priority !== 'high') return null;
  return (
    <span title={copy.row.highPriority} className="flex shrink-0 items-center text-warning">
      <Flag {...ICON_PROPS} size={14} />
      <span className="sr-only">{copy.row.highPriority}</span>
    </span>
  );
}
