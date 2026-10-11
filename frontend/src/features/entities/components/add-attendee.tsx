/**
 * "Add attendee": a search over everyone the user meets, below the
 * meeting's attendee list. Typing filters; choosing adds them as invitees.
 */
import { Command, CommandEmpty, CommandInput, CommandItem, CommandList } from '@/ui/command';

import { entitiesCopy } from '../copy';
import { usePeople } from '../queries';
import type { PersonId } from '../types';
import { EntityAvatar } from './entity-avatar';

/** Props of {@link AddAttendee}. */
export interface AddAttendeeProps {
  /** Who is already on the meeting, so they are not offered again. */
  present: readonly PersonId[];
  onAdd: (personId: PersonId) => void;
}

/** The search and its results. */
export function AddAttendee({ present, onAdd }: AddAttendeeProps) {
  const people = (usePeople().data ?? []).filter((person) => !present.includes(person.id));
  return (
    <Command className="mt-1 border-t border-border">
      <CommandInput placeholder={entitiesCopy.findPerson} aria-label={entitiesCopy.addAttendee} />
      <CommandList className="max-h-52">
        <CommandEmpty className="py-4 text-xs">{entitiesCopy.nobodyFound}</CommandEmpty>
        {people.map((person) => (
          <CommandItem
            key={person.id}
            value={`${person.name} ${person.email ?? ''} ${person.id}`}
            onSelect={() => {
              onAdd(person.id);
            }}
          >
            <EntityAvatar id={person.id} name={person.name} kind="person" size={20} />
            <span className="truncate">{person.name}</span>
            <span className="ml-auto truncate pl-2 text-xs text-fg-subtle">
              {person.companyName}
            </span>
          </CommandItem>
        ))}
      </CommandList>
    </Command>
  );
}
