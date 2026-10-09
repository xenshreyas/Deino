import { useQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { format } from 'date-fns';

import { useMeetings } from '@/features/meetings';
import type { Table } from '@/lib/ipc/generated/shared';
import { transcriptService } from '@/services/transcriptService';
import { CommandGroup, CommandItem, CommandShortcut } from '@/ui/command';
import { FileText, ICON_PROPS, NotebookText } from '@/ui/icons';

import { paletteCopy } from './copy';
import type { PaletteProviderProps } from './registry';

/** Transcript search starts at this many characters. */
const MIN_TRANSCRIPT_QUERY = 2;

/** How many meetings show before the user types. */
const RECENT_LIMIT = 6;

/** `Oct 8`, or an empty string for an unreadable timestamp. */
function shortDate(iso: string): string {
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? '' : format(date, 'MMM d');
}

/** "Meetings": open any meeting by title (the six most recent until you type). */
export function MeetingsProvider({ search, close }: PaletteProviderProps) {
  const navigate = useNavigate();
  const { data: meetings = [] } = useMeetings();
  const shown = search.trim() === '' ? meetings.slice(0, RECENT_LIMIT) : meetings;
  if (shown.length === 0) return null;
  return (
    <CommandGroup heading={paletteCopy.meetings}>
      {shown.map((meeting) => (
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

/** Tables the legacy transcript search reads. */
const TRANSCRIPT_TABLES = ['transcript_segments', 'meetings'] as const satisfies readonly Table[];

/** "In transcripts": meetings whose transcript mentions the search (until ORG-08). */
export function TranscriptsProvider({ search, close }: PaletteProviderProps) {
  const navigate = useNavigate();
  const query = search.trim();
  const { data: results = [] } = useQuery({
    queryKey: ['palette', 'transcript-search', query],
    queryFn: () => transcriptService.searchTranscripts(query),
    enabled: query.length >= MIN_TRANSCRIPT_QUERY,
    meta: { tables: TRANSCRIPT_TABLES },
  });
  if (query.length < MIN_TRANSCRIPT_QUERY || results.length === 0) return null;
  return (
    <CommandGroup heading={paletteCopy.transcripts}>
      {results.map((result) => (
        <CommandItem
          key={result.id}
          value={`transcript ${result.id}`}
          keywords={[query]}
          onSelect={() => {
            close();
            void navigate({ to: '/meetings/$meetingId', params: { meetingId: result.id } });
          }}
        >
          <FileText {...ICON_PROPS} />
          <span className="flex min-w-0 flex-col py-1.5">
            <span className="truncate">{result.title}</span>
            <span className="truncate text-xs text-fg-muted">{result.matchContext}</span>
          </span>
        </CommandItem>
      ))}
    </CommandGroup>
  );
}
