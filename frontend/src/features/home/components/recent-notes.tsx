/** Home's Recent notes: the five newest meetings, each opening its page. */
import { Link } from '@tanstack/react-router';

import { type Meeting, useMeetings } from '@/features/meetings';
import { SkeletonText } from '@/ui/skeleton';

import { homeCopy } from '../copy';
import { SectionHeading } from './section-heading';

/** How many notes Home lists. */
export const RECENT_NOTES = 5;

/** "Today 11:02", "Yesterday", or the date. */
function when(createdAt: string, now: Date): string {
  const at = new Date(createdAt);
  const days = Math.floor((now.setHours(0, 0, 0, 0) - new Date(at).setHours(0, 0, 0, 0)) / 864e5);
  if (days <= 0) return at.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  if (days === 1) return 'Yesterday';
  return at.toLocaleDateString([], { month: 'short', day: 'numeric' });
}

/** One note. */
function NoteRow({ meeting }: { meeting: Meeting }) {
  return (
    <li>
      <Link
        to="/meetings/$meetingId"
        params={{ meetingId: meeting.id }}
        className="flex h-10 items-center gap-3 rounded-md px-3 hover:bg-bg-muted"
      >
        <span className="min-w-0 flex-1 truncate text-sm text-fg">{meeting.title}</span>
        <span className="shrink-0 text-xs text-fg-muted tabular-nums">
          {when(meeting.createdAt, new Date())}
        </span>
      </Link>
    </li>
  );
}

/** The newest notes, or a one-line empty state. */
export function RecentNotes() {
  const meetings = useMeetings();
  const recent = meetings.data?.slice(0, RECENT_NOTES);
  return (
    <section aria-labelledby="home-recent" className="flex flex-col gap-1">
      <SectionHeading
        id="home-recent"
        action={
          <Link to="/meetings" className="text-xs text-fg-muted hover:text-fg">
            {homeCopy.viewAll}
          </Link>
        }
      >
        {homeCopy.recentNotes}
      </SectionHeading>
      {recent === undefined ? <SkeletonText lines={3} className="px-3" /> : null}
      {recent?.length === 0 ? (
        <p className="px-3 text-sm text-fg-muted">{homeCopy.noNotesBody}</p>
      ) : null}
      <ul>
        {recent?.map((meeting) => (
          <NoteRow key={meeting.id} meeting={meeting} />
        ))}
      </ul>
    </section>
  );
}
