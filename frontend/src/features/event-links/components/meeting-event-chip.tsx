/**
 * A meeting's calendar event in its header (Granola's event popover): the
 * linked event as a chip that opens the picker, or a ghost "Link event"
 * chip. It works the same while the meeting records (Circleback §3.5), on
 * the meeting page and on the record screen.
 */
import { useState } from 'react';
import { toast } from 'sonner';

import { Chip } from '@/ui/chip';
import { CalendarCheck, CalendarPlus } from '@/ui/icons';
import { Popover, PopoverContent, PopoverTrigger } from '@/ui/popover';

import { eventLinksCopy } from '../copy';
import { eventTitle } from '../format';
import { useEventSuggestions, useLinkEvent, useLinkedEvent } from '../queries';
import type { CalendarEvent, CalendarEventId, EventLinkChange, MeetingId } from '../types';
import { EventPicker } from './event-picker';

/** Says what a link or unlink did: the event, and who joined or left. */
function announce(change: EventLinkChange) {
  const description = eventLinksCopy.attendees(change.attendeesAdded, change.attendeesRemoved);
  const message =
    change.event === null
      ? eventLinksCopy.unlinked
      : eventLinksCopy.linkedTo(eventTitle(change.event));
  toast.success(message, description === undefined ? undefined : { description });
}

/** Links (or, for `null`, unlinks) the meeting's event, then says so. */
function useEventLinkActions(meetingId: MeetingId, close: () => void) {
  const mutation = useLinkEvent();
  const run = (eventId: CalendarEventId | null) => {
    close();
    mutation.mutate(
      { meetingId, eventId },
      {
        onSuccess: announce,
        onError: (error) => {
          const failed = eventId === null ? eventLinksCopy.unlinkFailed : eventLinksCopy.linkFailed;
          toast.error(failed, { description: error.message });
        },
      },
    );
  };
  return {
    pick: (event: CalendarEvent) => {
      run(event.id);
    },
    unlink: () => {
      run(null);
    },
  };
}

/** Props of {@link MeetingEventChip}. */
export interface MeetingEventChipProps {
  meetingId: MeetingId;
  /** Classes for the ghost "Link event" chip (the header reveals it on hover). */
  addChipClassName?: string | undefined;
}

/** The event chip and its picker; nothing until the link has loaded. */
export function MeetingEventChip({ meetingId, addChipClassName }: MeetingEventChipProps) {
  const [open, setOpen] = useState(false);
  const linked = useLinkedEvent(meetingId);
  const suggestions = useEventSuggestions(meetingId, open);
  const actions = useEventLinkActions(meetingId, () => {
    setOpen(false);
  });
  if (linked.data === undefined) return null;
  const event = linked.data;
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        {event === null ? (
          <Chip variant="ghost" icon={CalendarPlus} className={addChipClassName}>
            {eventLinksCopy.linkEvent}
          </Chip>
        ) : (
          <Chip
            icon={CalendarCheck}
            aria-label={`${eventLinksCopy.changeEvent}: ${eventTitle(event)}`}
          >
            <span className="max-w-60 truncate">{eventTitle(event)}</span>
          </Chip>
        )}
      </PopoverTrigger>
      <PopoverContent align="start" className="w-80 p-0">
        <EventPicker
          linked={event}
          suggestions={suggestions.data}
          failed={suggestions.isError}
          now={new Date()}
          onPick={actions.pick}
          onUnlink={actions.unlink}
        />
      </PopoverContent>
    </Popover>
  );
}
