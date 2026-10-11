/**
 * The calendar event picker (Granola §3.1, Circleback §3.5): the linked
 * event with Unlink, then the events around the meeting, closest first,
 * searchable. Presentational: the caller owns the data and the mutations.
 */
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/ui/command';
import { Check, ICON_PROPS, Unlink } from '@/ui/icons';
import { Skeleton } from '@/ui/skeleton';

import { eventLinksCopy } from '../copy';
import { eventTitle, eventWhen, guestNames } from '../format';
import type { CalendarEvent } from '../types';

/** Props of {@link EventPicker}. */
export interface EventPickerProps {
  /** The event the meeting is linked to now. */
  linked: CalendarEvent | null;
  /** The suggestions; `undefined` while they load. */
  suggestions: readonly CalendarEvent[] | undefined;
  /** The suggestions could not be loaded. */
  failed: boolean;
  /** The time the labels are relative to. */
  now: Date;
  onPick: (event: CalendarEvent) => void;
  onUnlink: () => void;
}

/** One event: title, then when and how many people, with a check when linked. */
function EventOption({
  event,
  now,
  checked,
  onPick,
}: {
  event: CalendarEvent;
  now: Date;
  checked: boolean;
  onPick: (event: CalendarEvent) => void;
}) {
  const people = guestNames(event).length;
  const title = eventTitle(event);
  return (
    <CommandItem
      value={`event ${event.id}`}
      keywords={[title]}
      onSelect={() => {
        onPick(event);
      }}
      className="py-1.5"
    >
      <span className="flex min-w-0 flex-1 flex-col">
        <span className="truncate text-fg">{title}</span>
        <span className="truncate text-xs text-fg-muted tabular-nums">
          {eventWhen(event, now)}
          {people > 0 ? ` · ${eventLinksCopy.people(people)}` : ''}
        </span>
      </span>
      {checked ? <Check {...ICON_PROPS} className="shrink-0 text-accent" /> : null}
    </CommandItem>
  );
}

/** Three quiet rows while the suggestions load. */
function SuggestionsSkeleton() {
  return (
    <div className="flex flex-col gap-3 px-2.5 py-2" aria-busy>
      {[0, 1, 2].map((row) => (
        <div key={row} className="flex flex-col gap-1.5">
          <Skeleton className="h-3 w-3/5" />
          <Skeleton className="h-2.5 w-2/5" />
        </div>
      ))}
    </div>
  );
}

/** A quiet line in place of the suggestions. */
function Note({ children }: { children: string }) {
  return <p className="px-2.5 py-3 text-sm text-pretty text-fg-muted">{children}</p>;
}

/** The linked event and Unlink. */
function LinkedGroup({
  linked,
  now,
  onPick,
  onUnlink,
}: EventPickerProps & { linked: CalendarEvent }) {
  return (
    <CommandGroup heading={eventLinksCopy.linkedGroup}>
      <EventOption event={linked} now={now} checked onPick={onPick} />
      <CommandItem value="unlink" keywords={[eventLinksCopy.unlink]} onSelect={onUnlink}>
        <Unlink {...ICON_PROPS} className="text-fg-muted" />
        <span>{eventLinksCopy.unlink}</span>
      </CommandItem>
    </CommandGroup>
  );
}

/** The events around the meeting, the linked one left out. */
function SuggestedGroup({ linked, suggestions, failed, now, onPick }: EventPickerProps) {
  if (failed) return <Note>{eventLinksCopy.loadFailed}</Note>;
  if (suggestions === undefined) return <SuggestionsSkeleton />;
  const others = suggestions.filter((event) => event.id !== linked?.id);
  if (others.length === 0) return <Note>{eventLinksCopy.noSuggestions}</Note>;
  return (
    <CommandGroup heading={eventLinksCopy.suggestedGroup}>
      {others.map((event) => (
        <EventOption key={event.id} event={event} now={now} checked={false} onPick={onPick} />
      ))}
    </CommandGroup>
  );
}

/** The searchable picker. */
export function EventPicker(props: EventPickerProps) {
  // cmdk shows Empty whenever no row matches, so only while searching rows.
  const hasRows = props.linked !== null || (props.suggestions ?? []).length > 0;
  return (
    <Command loop label={eventLinksCopy.pickerLabel}>
      <CommandInput placeholder={eventLinksCopy.searchPlaceholder} />
      <CommandList className="max-h-80">
        {hasRows ? <CommandEmpty>{eventLinksCopy.noMatch}</CommandEmpty> : null}
        {props.linked === null ? null : <LinkedGroup {...props} linked={props.linked} />}
        <SuggestedGroup {...props} />
      </CommandList>
    </Command>
  );
}
