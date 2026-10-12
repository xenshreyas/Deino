/**
 * A Coming up row's ⋯ menu (also opened by right-clicking the row): open
 * the event's note (a scheduled meeting linked to the event, NOT-01 +
 * ORG-03), email the invitees, open it in Calendar, and review the last
 * meeting (the one before it in its series, or with the same people).
 */
import { useNavigate } from '@tanstack/react-router';
import { toast } from 'sonner';

import { type CalendarEvent, useLastMeeting, useOpenInCalendar } from '@/features/calendar';
import { IconButton } from '@/ui/icon-button';
import { Ellipsis } from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
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

/** Opens the meeting to review before `event`, or says there is none. */
function useReviewLast(event: CalendarEvent) {
  const navigate = useNavigate();
  const last = useLastMeeting();
  return () => {
    last.mutate(event.id, {
      onSuccess: (meeting) => {
        if (meeting === null) {
          toast(homeCopy.noLastMeeting);
          return;
        }
        void navigate({ to: '/meetings/$meetingId', params: { meetingId: meeting.id } });
      },
      onError: () => toast.error(homeCopy.lastMeetingFailed),
    });
  };
}

/** Props of {@link EventMenu}. */
export interface EventMenuProps {
  event: CalendarEvent;
  /** Controlled, so a right-click on the row opens the same menu. */
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

/** The ⋯ menu of one event. */
export function EventMenu({ event, open, onOpenChange }: EventMenuProps) {
  const openNote = useOpenNote(event);
  const reviewLast = useReviewLast(event);
  const openInCalendar = useOpenInCalendar();
  const mailto = inviteesMailto(event);
  return (
    <DropdownMenu open={open} onOpenChange={onOpenChange}>
      <DropdownMenuTrigger asChild>
        <IconButton icon={Ellipsis} label={homeCopy.more} />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem onSelect={() => void openNote()}>{homeCopy.openNote}</DropdownMenuItem>
        <DropdownMenuItem onSelect={reviewLast}>{homeCopy.reviewLast}</DropdownMenuItem>
        <DropdownMenuSeparator />
        {mailto === null ? null : (
          <DropdownMenuItem asChild>
            <a href={mailto}>{homeCopy.emailInvitees}</a>
          </DropdownMenuItem>
        )}
        <DropdownMenuItem
          onSelect={() => {
            openInCalendar.mutate(event.id, {
              onError: () => toast.error(homeCopy.openInCalendarFailed),
            });
          }}
        >
          {homeCopy.openInCalendar}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
