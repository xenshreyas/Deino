/**
 * The assignee chip and its picker: a popover with a searchable list of
 * people (the user first), plus "Unassigned". Used on rows and in the bulk
 * bar ("Assign").
 */
import { type ReactNode, useState } from 'react';

import { cn } from '@/ui/cn';
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/ui/command';
import { Check, ICON_PROPS, UserRound } from '@/ui/icons';
import { Popover, PopoverContent, PopoverTrigger } from '@/ui/popover';

import { copy } from '../copy';
import type { PersonId, PersonOption } from '../types';

/** Props of {@link AssigneePicker}. */
export interface AssigneePickerProps {
  people: readonly PersonOption[];
  /** The current assignee; leave it out where there is none to mark (bulk). */
  value?: PersonId | null | undefined;
  onChange: (person: PersonId | null) => void;
  /** The element that opens the picker (a chip or a button). */
  children: ReactNode;
}

/** Up to two initials of a name, for the 16px avatar. */
export function initials(name: string): string {
  const words = name.trim().split(/\s+/u);
  return words
    .slice(0, 2)
    .map((word) => word.charAt(0).toUpperCase())
    .join('');
}

/** A 16px initials avatar. */
export function Avatar({ name, className }: { name: string; className?: string }) {
  return (
    <span
      aria-hidden="true"
      className={cn(
        'flex size-4 shrink-0 items-center justify-center rounded-full bg-bg-muted text-[9px] font-semibold text-fg-muted',
        className,
      )}
    >
      {initials(name)}
    </span>
  );
}

/** One person in the list. */
function PersonItem({
  person,
  selected,
  onSelect,
}: {
  person: PersonOption;
  selected: boolean;
  onSelect: () => void;
}) {
  const name = person.isMe ? copy.assignee.me : person.name;
  return (
    <CommandItem value={`${person.name} ${person.email ?? ''}`} onSelect={onSelect}>
      <Avatar name={person.name} />
      <span className="min-w-0 flex-1 truncate">{name}</span>
      {selected ? <Check {...ICON_PROPS} className="text-accent" /> : null}
    </CommandItem>
  );
}

/** The searchable list: people, then "Unassigned". */
function PeopleList({
  people,
  value,
  onPick,
}: {
  people: readonly PersonOption[];
  value: PersonId | null | undefined;
  onPick: (person: PersonId | null) => void;
}) {
  return (
    <Command>
      <CommandInput placeholder={copy.assignee.search} aria-label={copy.assignee.search} />
      <CommandList className="max-h-72">
        <CommandEmpty>{copy.assignee.nobody}</CommandEmpty>
        <CommandGroup>
          {people.map((person) => (
            <PersonItem
              key={person.id}
              person={person}
              selected={person.id === value}
              onSelect={() => {
                onPick(person.id);
              }}
            />
          ))}
          <CommandItem
            value={copy.assignee.unassigned}
            onSelect={() => {
              onPick(null);
            }}
          >
            <UserRound {...ICON_PROPS} className="text-fg-subtle" />
            <span className="flex-1">{copy.assignee.unassigned}</span>
            {value === null ? <Check {...ICON_PROPS} className="text-accent" /> : null}
          </CommandItem>
        </CommandGroup>
      </CommandList>
    </Command>
  );
}

/** The popover with the searchable list. */
export function AssigneePicker({ people, value, onChange, children }: AssigneePickerProps) {
  const [open, setOpen] = useState(false);
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>{children}</PopoverTrigger>
      <PopoverContent
        align="end"
        className="w-64 p-0"
        onClick={(event) => {
          event.stopPropagation();
        }}
      >
        <PeopleList
          people={people}
          value={value}
          onPick={(person) => {
            setOpen(false);
            onChange(person);
          }}
        />
      </PopoverContent>
    </Popover>
  );
}
