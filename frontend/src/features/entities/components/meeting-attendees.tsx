/**
 * The meeting header's attendee chip (ui.md §3 Meeting header): up to three
 * avatars and a "+N", or a ghost "Add attendees" chip on an empty meeting.
 * It opens the attendee list, where each name has its hover card and opens
 * its page, anyone can be taken off, and anyone can be added.
 */
import { Link } from '@tanstack/react-router';
import { useState } from 'react';

import { Chip } from '@/ui/chip';
import { ICON_PROPS, Users, X } from '@/ui/icons';
import { Popover, PopoverContent, PopoverTrigger } from '@/ui/popover';

import { entitiesCopy } from '../copy';
import { useAttendance } from '../mutations';
import { useMeetingAttendees } from '../queries';
import type { MeetingId, MeetingPerson } from '../types';
import { AddAttendee } from './add-attendee';
import { EntityAvatar } from './entity-avatar';
import { PersonHoverCard } from './person-hover-card';

/** Avatars the chip stacks before "+N". */
const STACKED = 3;

/** The organiser, a speaker who was not invited, or the user. */
function roleNote(person: MeetingPerson): string | null {
  if (person.isMe) return entitiesCopy.you;
  if (person.role === 'organizer') return entitiesCopy.organizer;
  return person.role === 'speaker_only' ? entitiesCopy.speakerOnly : null;
}

/** One attendee: a link to their page with their card, and a remove button. */
function AttendeeRow({ person, onRemove }: { person: MeetingPerson; onRemove: () => void }) {
  return (
    <li className="group flex h-8 items-center rounded-sm hover:bg-bg-muted">
      <PersonHoverCard personId={person.personId} side="right">
        <Link
          to="/people/$personId"
          params={{ personId: person.personId }}
          className="flex h-full min-w-0 flex-1 items-center gap-2 px-2 text-sm"
        >
          <EntityAvatar
            id={person.personId}
            name={person.name}
            kind="person"
            logoPath={person.companyLogoPath}
            size={20}
          />
          <span className="truncate text-fg">{person.name}</span>
          <span className="truncate text-xs text-fg-subtle">{roleNote(person)}</span>
        </Link>
      </PersonHoverCard>
      <button
        type="button"
        aria-label={entitiesCopy.removeAttendee(person.name)}
        onClick={onRemove}
        className="mr-1 flex size-6 shrink-0 items-center justify-center rounded-sm text-fg-muted opacity-0 group-hover:opacity-100 hover:bg-bg hover:text-fg focus-visible:opacity-100"
      >
        <X {...ICON_PROPS} size={14} />
      </button>
    </li>
  );
}

/** The chip's face: stacked avatars and a count, or the resting label. */
function ChipFace({ people }: { people: readonly MeetingPerson[] }) {
  const others = people.filter((person) => !person.isMe);
  if (others.length === 0) return <>{entitiesCopy.addAttendees}</>;
  return (
    <>
      <span className="flex -space-x-1">
        {others.slice(0, STACKED).map((person) => (
          <EntityAvatar
            key={person.personId}
            id={person.personId}
            name={person.name}
            kind="person"
            size={20}
            className="size-4 text-[8px] ring-2 ring-bg"
          />
        ))}
      </span>
      <span className="truncate">
        {others.length === 1 ? others[0]?.name : entitiesCopy.peopleCount(others.length)}
      </span>
    </>
  );
}

/** Props of {@link MeetingAttendees}. */
export interface MeetingAttendeesProps {
  meetingId: MeetingId;
  /** Classes for the resting chip (the header reveals it on hover). */
  restingClassName?: string;
}

/** The list inside the popover: everyone on the meeting, then "Add attendee". */
function AttendeeList({
  meetingId,
  people,
}: {
  meetingId: MeetingId;
  people: readonly MeetingPerson[];
}) {
  const { add, remove } = useAttendance(meetingId);
  return (
    <>
      {people.length === 0 ? null : (
        <ul aria-label={entitiesCopy.attendees} className="flex max-h-64 flex-col overflow-y-auto">
          {people.map((person) => (
            <AttendeeRow
              key={person.personId}
              person={person}
              onRemove={() => {
                remove.mutate(person.personId);
              }}
            />
          ))}
        </ul>
      )}
      <AddAttendee
        present={people.map((person) => person.personId)}
        onAdd={(personId) => {
          add.mutate(personId);
        }}
      />
    </>
  );
}

/** The attendee chip and its list. */
export function MeetingAttendees({ meetingId, restingClassName }: MeetingAttendeesProps) {
  const attendees = useMeetingAttendees(meetingId);
  const [open, setOpen] = useState(false);
  const people = attendees.data ?? [];
  const resting = people.every((person) => person.isMe);
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Chip
          variant={resting ? 'ghost' : 'outline'}
          icon={resting ? Users : undefined}
          className={resting ? restingClassName : undefined}
        >
          <ChipFace people={people} />
        </Chip>
      </PopoverTrigger>
      <PopoverContent align="start" className="w-72 p-1">
        <AttendeeList meetingId={meetingId} people={people} />
      </PopoverContent>
    </Popover>
  );
}
