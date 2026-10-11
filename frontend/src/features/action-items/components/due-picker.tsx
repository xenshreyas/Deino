/**
 * The due-date picker: quick choices (today, tomorrow, next week, none) and
 * any day through the system date field. Days are local `YYYY-MM-DD`; Rust
 * stores them as noon on that day.
 */
import { type ReactNode, useState } from 'react';

import { cn } from '@/ui/cn';
import { Check, ICON_PROPS } from '@/ui/icons';
import { fieldClassName } from '@/ui/input';
import { Popover, PopoverContent, PopoverTrigger } from '@/ui/popover';

import { copy } from '../copy';
import { quickDueChoices } from '../dates';

/** Props of {@link DuePicker}. */
export interface DuePickerProps {
  /** The current due day (`YYYY-MM-DD`) or `null`; left out where there is none to mark (bulk). */
  value?: string | null | undefined;
  onChange: (day: string | null) => void;
  /** The element that opens the picker. */
  children: ReactNode;
}

/** One quick choice. */
function Choice({
  label,
  selected,
  onSelect,
}: {
  label: string;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      className={cn(
        'flex h-[30px] w-full items-center gap-2 rounded-sm px-2 text-left text-sm text-fg',
        'transition-colors duration-(--dur-fast) hover:bg-bg-muted focus-visible:bg-bg-muted focus-visible:outline-none',
      )}
    >
      <span className="flex-1">{label}</span>
      {selected ? <Check {...ICON_PROPS} className="text-accent" /> : null}
    </button>
  );
}

/** Any day, through the system date field. */
function DayField({
  value,
  onPick,
}: {
  value: string | null | undefined;
  onPick: (day: string) => void;
}) {
  return (
    <label className="mt-1 flex flex-col gap-1 border-t border-border px-2 pt-2 pb-1 text-xs text-fg-muted">
      {copy.due.pick}
      <input
        type="date"
        value={value ?? ''}
        onChange={(event) => {
          if (event.target.value !== '') onPick(event.target.value);
        }}
        className={cn(fieldClassName, 'h-8 px-2 tabular-nums')}
      />
    </label>
  );
}

/** The popover with quick choices and a date field. */
export function DuePicker({ value, onChange, children }: DuePickerProps) {
  const [open, setOpen] = useState(false);
  const pick = (day: string | null) => {
    setOpen(false);
    onChange(day);
  };
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>{children}</PopoverTrigger>
      <PopoverContent
        align="end"
        className="w-56 p-1"
        onClick={(event) => {
          event.stopPropagation();
        }}
      >
        {quickDueChoices(new Date()).map((choice) => (
          <Choice
            key={choice.label}
            label={choice.label}
            selected={choice.day === value}
            onSelect={() => {
              pick(choice.day);
            }}
          />
        ))}
        <DayField value={value} onPick={pick} />
      </PopoverContent>
    </Popover>
  );
}
