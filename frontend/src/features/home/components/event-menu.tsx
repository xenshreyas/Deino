/**
 * A Coming up row's ⋯ menu: open the event's note (a scheduled meeting
 * linked to the event, NOT-01 + ORG-03) and email the invitees.
 */
import { useNavigate } from '@tanstack/react-router';
import { toast } from 'sonner';

import type { CalendarEvent } from '@/features/calendar';
import { IconButton } from '@/ui/icon-button';
import { Ellipsis } from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/ui/menu';

import { homeApi } from '../api';
import { homeCopy } from '../copy';
import { inviteesMailto } from '../event-actions';

/** Creates the event's scheduled note, links it, and opens it. */
function useOpenNote(event: CalendarEvent) {
  const navigate = useNavigate();
  return async () => {
    try {
      const meetingId = await homeApi.openEventNote(event);
      await navigate({ to: '/meetings/$meetingId', params: { meetingId } });
    } catch {
      toast.error(homeCopy.openNoteFailed);
    }
  };
}

/** The ⋯ menu of one event. */
export function EventMenu({ event }: { event: CalendarEvent }) {
  const openNote = useOpenNote(event);
  const mailto = inviteesMailto(event);
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <IconButton icon={Ellipsis} label={homeCopy.more} />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem onSelect={() => void openNote()}>{homeCopy.openNote}</DropdownMenuItem>
        {mailto === null ? null : (
          <DropdownMenuItem asChild>
            <a href={mailto}>{homeCopy.emailInvitees}</a>
          </DropdownMenuItem>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
