/**
 * The meetings of a folder or of Unfiled, as 40px list rows (ui.md §3 List
 * row): icon, title, date, and an optional ⋯ action. Rows drag onto the
 * sidebar's folders like the Meetings page's rows.
 */
import { Link } from '@tanstack/react-router';
import { format } from 'date-fns';
import type { ReactNode } from 'react';

import { NotebookText, ICON_PROPS } from '@/ui/icons';
import { Skeleton } from '@/ui/skeleton';

import { meetingDragSource } from '../dnd';
import type { Meeting } from '../types';

/** `Oct 8, 2026`, or nothing for an unreadable timestamp. */
export function meetingDate(iso: string): string {
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? '' : format(date, 'MMM d, yyyy');
}

/** Props of {@link FolderMeetingList}. */
export interface FolderMeetingListProps {
  meetings: readonly Meeting[];
  /** The trailing action of a row (a ⋯ menu), if any. */
  action?: ((meeting: Meeting) => ReactNode) | undefined;
}

/** One meeting row. */
function MeetingRow({ meeting, action }: { meeting: Meeting; action: ReactNode }) {
  return (
    <li
      {...meetingDragSource(meeting)}
      className="group relative flex h-10 items-center rounded-md transition-colors duration-(--dur-fast) hover:bg-bg-muted has-[a:focus-visible]:bg-bg-muted"
    >
      <Link
        to="/meetings/$meetingId"
        params={{ meetingId: meeting.id }}
        draggable={false}
        className="flex h-full min-w-0 flex-1 items-center gap-3 rounded-md pr-10 pl-3 focus-visible:outline-offset-[-2px]"
      >
        <NotebookText {...ICON_PROPS} className="shrink-0 text-fg-muted" />
        <span className="min-w-0 flex-1 truncate text-sm text-fg">{meeting.title}</span>
        <span className="shrink-0 text-xs text-fg-muted tabular-nums">
          {meetingDate(meeting.createdAt)}
        </span>
      </Link>
      {action === null ? null : <div className="absolute right-1.5">{action}</div>}
    </li>
  );
}

/** The rows, newest first. */
export function FolderMeetingList({ meetings, action }: FolderMeetingListProps) {
  return (
    <ul className="flex flex-col">
      {meetings.map((meeting) => (
        <MeetingRow key={meeting.id} meeting={meeting} action={action?.(meeting) ?? null} />
      ))}
    </ul>
  );
}

/** Row-shaped placeholders while meetings load. */
export function MeetingListSkeleton() {
  return (
    <div role="status" aria-label="Loading" className="flex flex-col gap-1">
      {Array.from({ length: 4 }, (_, index) => (
        <div key={index} className="flex h-10 items-center gap-3 px-3">
          <Skeleton className="size-4 rounded-full" />
          <Skeleton className="h-2.5 flex-1" />
          <Skeleton className="h-2.5 w-16" />
        </div>
      ))}
    </div>
  );
}
