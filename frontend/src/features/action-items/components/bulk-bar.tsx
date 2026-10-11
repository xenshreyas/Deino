/**
 * The bulk action bar (Circleback §8 "Bulk selection"): floats at the
 * bottom of the page while items are selected, with status, reassign, due
 * date and delete for all of them at once. A floating layer, so it carries
 * the chat bar's shadow (ui.md §3 Chat bar).
 */
import { Button } from '@/ui/button';
import { Calendar, CheckCheck, ICON_PROPS, RotateCcw, Trash2, UserRound, X } from '@/ui/icons';
import { IconButton } from '@/ui/icon-button';

import { copy } from '../copy';
import type { ActionItemEdit, PersonOption } from '../types';
import { AssigneePicker } from './assignee-picker';
import { DuePicker } from './due-picker';

/** Props of {@link BulkBar}. */
export interface BulkBarProps {
  count: number;
  people: readonly PersonOption[];
  /** A status, assignee or due-date edit for every selected item. */
  onEdit: (edit: ActionItemEdit) => void;
  onDelete: () => void;
  onClear: () => void;
}

/** "Mark done" and "Reopen". */
function StatusButtons({ onEdit }: Pick<BulkBarProps, 'onEdit'>) {
  return (
    <>
      <Button
        variant="ghost"
        size="sm"
        onClick={() => {
          onEdit({ field: 'status', status: 'done' });
        }}
      >
        <CheckCheck {...ICON_PROPS} />
        {copy.bulk.done}
      </Button>
      <Button
        variant="ghost"
        size="sm"
        onClick={() => {
          onEdit({ field: 'status', status: 'open' });
        }}
      >
        <RotateCcw {...ICON_PROPS} />
        {copy.bulk.reopen}
      </Button>
    </>
  );
}

/** "Assign" and "Due date", each with its picker. */
function PickerButtons({ people, onEdit }: Pick<BulkBarProps, 'people' | 'onEdit'>) {
  return (
    <>
      <AssigneePicker
        people={people}
        onChange={(personId) => {
          onEdit({ field: 'assignee', personId });
        }}
      >
        <Button variant="ghost" size="sm">
          <UserRound {...ICON_PROPS} />
          {copy.bulk.assign}
        </Button>
      </AssigneePicker>
      <DuePicker
        onChange={(date) => {
          onEdit({ field: 'due', date });
        }}
      >
        <Button variant="ghost" size="sm">
          <Calendar {...ICON_PROPS} />
          {copy.bulk.due}
        </Button>
      </DuePicker>
    </>
  );
}

/** The bar, or nothing when nothing is selected. */
export function BulkBar({ count, people, onEdit, onDelete, onClear }: BulkBarProps) {
  if (count === 0) return null;
  return (
    <div
      role="toolbar"
      aria-label={copy.bulk.label}
      className="fixed bottom-6 left-1/2 z-(--z-chatbar) flex h-11 -translate-x-1/2 animate-in items-center gap-1 rounded-full bg-surface pr-1.5 pl-4 shadow-chatbar duration-(--dur-base) ease-out fade-in-0 slide-in-from-bottom-2 motion-reduce:animate-none"
    >
      <span className="mr-2 text-sm font-semibold whitespace-nowrap text-fg tabular-nums">
        {copy.bulk.selected(count)}
      </span>
      <StatusButtons onEdit={onEdit} />
      <PickerButtons people={people} onEdit={onEdit} />
      <Button variant="danger" size="sm" onClick={onDelete}>
        <Trash2 {...ICON_PROPS} />
        {copy.bulk.delete}
      </Button>
      <span aria-hidden="true" className="mx-1 h-5 w-px bg-border" />
      <IconButton icon={X} label={copy.bulk.clear} shortcut="Esc" onClick={onClear} />
    </div>
  );
}
