/**
 * The name list of the "Assign speaker" popover: the meeting's attendees,
 * then everyone else in People, then "Add “…”" for someone new. The server
 * filters by the search, so cmdk only drives the keyboard. Presentational.
 */
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/ui/command';
import { ICON_PROPS, Plus } from '@/ui/icons';

import { speakersCopy } from '../copy';
import type { NameCandidate, SpeakerName } from '../types';
import { SpeakerAvatar } from './speaker-avatar';

/** Props of {@link NamePicker}. */
export interface NamePickerProps {
  candidates: readonly NameCandidate[];
  search: string;
  onSearch: (search: string) => void;
  onPick: (name: SpeakerName) => void;
}

/** True when `name` is already one of `candidates`, ignoring case and spaces. */
function isListed(candidates: readonly NameCandidate[], name: string): boolean {
  const wanted = name.trim().toLowerCase();
  return candidates.some((candidate) => candidate.person.name.toLowerCase() === wanted);
}

/** One person: avatar, name, and "You" for the user. */
function CandidateItem({
  candidate,
  onPick,
}: {
  candidate: NameCandidate;
  onPick: (name: SpeakerName) => void;
}) {
  const { person } = candidate;
  return (
    <CommandItem
      value={`person ${person.id}`}
      onSelect={() => {
        onPick({ kind: 'person', personId: person.id });
      }}
    >
      <SpeakerAvatar name={person.name} />
      <span className="min-w-0 flex-1 truncate">{person.name}</span>
      {candidate.isMe ? (
        <span className="text-xs text-fg-muted">{speakersCopy.meBadge}</span>
      ) : null}
    </CommandItem>
  );
}

/** A labelled group, hidden when empty. */
function CandidateGroup({
  heading,
  candidates,
  onPick,
}: {
  heading: string;
  candidates: readonly NameCandidate[];
  onPick: (name: SpeakerName) => void;
}) {
  if (candidates.length === 0) return null;
  return (
    <CommandGroup heading={heading}>
      {candidates.map((candidate) => (
        <CandidateItem key={candidate.person.id} candidate={candidate} onPick={onPick} />
      ))}
    </CommandGroup>
  );
}

/** Search, pick someone, or add a new person. */
export function NamePicker({ candidates, search, onSearch, onPick }: NamePickerProps) {
  const name = search.trim();
  const canCreate = name !== '' && !isListed(candidates, name);
  return (
    <Command loop shouldFilter={false}>
      <CommandInput
        value={search}
        onValueChange={onSearch}
        placeholder={speakersCopy.searchPlaceholder}
      />
      <CommandList className="max-h-72">
        <CommandEmpty>{speakersCopy.noMatches}</CommandEmpty>
        <CandidateGroup
          heading={speakersCopy.inMeeting}
          candidates={candidates.filter((candidate) => candidate.isAttendee)}
          onPick={onPick}
        />
        <CandidateGroup
          heading={speakersCopy.people}
          candidates={candidates.filter((candidate) => !candidate.isAttendee)}
          onPick={onPick}
        />
        {canCreate ? (
          <CommandGroup>
            <CommandItem
              value={`create ${name}`}
              onSelect={() => {
                onPick({ kind: 'new', name });
              }}
            >
              <Plus {...ICON_PROPS} />
              <span className="truncate">{speakersCopy.createNamed(name)}</span>
            </CommandItem>
          </CommandGroup>
        ) : null}
      </CommandList>
    </Command>
  );
}
