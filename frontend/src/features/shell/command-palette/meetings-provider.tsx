import { useNavigate } from '@tanstack/react-router';
import { format } from 'date-fns';

import { useMeetings } from '@/features/meetings';
import { CommandGroup, CommandItem, CommandShortcut } from '@/ui/command';
import { ICON_PROPS, NotebookText } from '@/ui/icons';

import { paletteCopy } from './copy';
import type { PaletteProviderProps } from './registry';

/** How many meetings show before the user types. */
const RECENT_LIMIT = 6;

/** `Oct 8`, or an empty string for an unreadable timestamp. */
function shortDate(iso: string): string {
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? '' : format(date, 'MMM d');
}

/**
 * "Meetings": the six most recent meetings while the palette is empty. Once
 * the user types or filters, full-text search (`features/search`) answers.
 */
export function MeetingsProvider({ search, filters, close }: PaletteProviderProps) {
  const navigate = useNavigate();
  const { data: meetings = [] } = useMeetings();
  if (search.trim() !== '' || filters.length > 0 || meetings.length === 0) return null;
  return (
    <CommandGroup heading={paletteCopy.meetings}>
      {meetings.slice(0, RECENT_LIMIT).map((meeting) => (
        <CommandItem
          key={meeting.id}
          value={`meeting ${meeting.id} ${meeting.title}`}
          keywords={[meeting.title]}
          onSelect={() => {
            close();
            void navigate({ to: '/meetings/$meetingId', params: { meetingId: meeting.id } });
          }}
        >
          <NotebookText {...ICON_PROPS} />
          <span className="truncate">{meeting.title}</span>
          <CommandShortcut>{shortDate(meeting.createdAt)}</CommandShortcut>
        </CommandItem>
      ))}
    </CommandGroup>
  );
}
