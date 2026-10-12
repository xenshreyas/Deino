/**
 * The filter bar under the tabs (ui.md §3 Page header, "filter bar"): the
 * active conditions as 28px chips, each removable, and "Filter" (`F`), which
 * opens a searchable menu of every condition (Circleback §14: "`F` opens
 * keyboard filter search").
 */
import { useMemo } from 'react';

import { cn } from '@/ui/cn';
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/ui/command';
import { ICON_PROPS, ListFilter, X } from '@/ui/icons';
import { Kbd } from '@/ui/kbd';
import { Popover, PopoverContent, PopoverTrigger } from '@/ui/popover';

import { copy } from '../copy';
import { activeChips, type FilterChoice, filterChoices, without } from '../filters';
import type { FilterOptions, MeetingFilter } from '../types';

/** Props of {@link FilterBar}. */
export interface FilterBarProps {
  filter: MeetingFilter;
  options: FilterOptions;
  onChange: (filter: MeetingFilter) => void;
  /** Whether the menu is open; the page opens it on `F`. */
  menuOpen: boolean;
  onMenuOpenChange: (open: boolean) => void;
}

const CHIP =
  'inline-flex h-7 items-center gap-1 rounded-full border text-xs whitespace-nowrap transition-colors duration-(--dur-fast) ease-out';

/** One active condition, with its remove button. */
function ConditionChip({ label, onRemove }: { label: string; onRemove: () => void }) {
  return (
    <span className={cn(CHIP, 'max-w-72 border-transparent bg-accent-soft pr-1 pl-3 text-fg')}>
      <span className="truncate">{label}</span>
      <button
        type="button"
        aria-label={copy.filter.remove(label)}
        onClick={onRemove}
        className="flex size-5 shrink-0 items-center justify-center rounded-full text-fg-muted hover:bg-bg-muted hover:text-fg focus-visible:outline-2 focus-visible:outline-focus-ring"
      >
        <X {...ICON_PROPS} size={12} />
      </button>
    </span>
  );
}

/** The menu's groups, in the order the choices come. */
function grouped(choices: readonly FilterChoice[]): [string, FilterChoice[]][] {
  const groups = new Map<string, FilterChoice[]>();
  for (const choice of choices)
    groups.set(choice.group, [...(groups.get(choice.group) ?? []), choice]);
  return [...groups.entries()];
}

/** The searchable list of every condition. */
function FilterMenu({
  filter,
  options,
  onChange,
  onMenuOpenChange,
}: Omit<FilterBarProps, 'menuOpen'>) {
  const groups = useMemo(() => grouped(filterChoices(options, new Date())), [options]);
  return (
    <Command>
      <CommandInput placeholder={copy.filter.search} aria-label={copy.filter.search} />
      <CommandList className="max-h-80">
        <CommandEmpty>{copy.filter.nothing}</CommandEmpty>
        {groups.map(([group, choices]) => (
          <CommandGroup key={group} heading={group}>
            {choices.map((choice) => (
              <CommandItem
                key={choice.id}
                value={`${group} ${choice.label} ${choice.id}`}
                onSelect={() => {
                  onMenuOpenChange(false);
                  onChange(choice.apply(filter));
                }}
              >
                {choice.label}
              </CommandItem>
            ))}
          </CommandGroup>
        ))}
      </CommandList>
    </Command>
  );
}

/** The chips and the "Filter" button with its menu. */
export function FilterBar(props: FilterBarProps) {
  const { filter, options, onChange, menuOpen, onMenuOpenChange } = props;
  return (
    <div
      className="flex min-h-8 flex-wrap items-center gap-1.5 py-3"
      role="toolbar"
      aria-label={copy.filter.button}
    >
      {activeChips(filter, options).map((chip) => (
        <ConditionChip
          key={chip.field}
          label={chip.label}
          onRemove={() => {
            onChange(without(filter, chip.field));
          }}
        />
      ))}
      <Popover open={menuOpen} onOpenChange={onMenuOpenChange}>
        <PopoverTrigger asChild>
          <button
            type="button"
            className={cn(
              CHIP,
              'gap-1.5 border-border px-3 text-fg-muted hover:bg-bg-muted hover:text-fg data-[state=open]:bg-bg-muted',
            )}
          >
            <ListFilter {...ICON_PROPS} size={14} />
            {copy.filter.button}
            <Kbd className="h-4 min-w-4 border-none">F</Kbd>
          </button>
        </PopoverTrigger>
        <PopoverContent align="start" className="w-72 p-0">
          <FilterMenu {...props} />
        </PopoverContent>
      </Popover>
    </div>
  );
}
