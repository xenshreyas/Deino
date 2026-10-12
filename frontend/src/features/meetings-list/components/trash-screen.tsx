/**
 * The Trash (Granola §3.6): meetings deleted in the last 30 days, newest
 * deletion first, each with Restore and Delete forever, and how long it has
 * left before the purge deletes it and its recordings.
 */
import { differenceInCalendarDays } from 'date-fns';
import { useState } from 'react';

import { Button } from '@/ui/button';
import { EmptyState } from '@/ui/empty-state';
import { ICON_PROPS, RotateCcw, Trash2 } from '@/ui/icons';
import { Page, PageHeader } from '@/ui/page-header';
import { Skeleton } from '@/ui/skeleton';

import { deleteForever, restoreMeetings } from '../actions';
import { copy } from '../copy';
import { useTrash } from '../queries';
import type { MeetingId, TrashedMeeting } from '../types';
import { ConfirmDialog } from './confirm-dialog';

/** One trashed meeting's row. */
function TrashRow({
  meeting,
  onDelete,
}: {
  meeting: TrashedMeeting;
  onDelete: (id: MeetingId) => void;
}) {
  const days = differenceInCalendarDays(new Date(meeting.purgesAt), new Date());
  return (
    <li className="group flex h-10 items-center gap-3 rounded-md px-3 hover:bg-bg-muted">
      <span className="min-w-0 flex-1 truncate text-sm text-fg">{meeting.title}</span>
      <span className="shrink-0 text-xs text-fg-subtle tabular-nums">
        {copy.trash.purgesIn(days)}
      </span>
      <Button variant="ghost" size="sm" onClick={() => void restoreMeetings([meeting.id])}>
        <RotateCcw {...ICON_PROPS} />
        {copy.trash.restore}
      </Button>
      <Button
        variant="danger"
        size="sm"
        onClick={() => {
          onDelete(meeting.id);
        }}
      >
        {copy.trash.deleteForever}
      </Button>
    </li>
  );
}

/** The rows, or the empty state. */
function TrashList({
  meetings,
  onDelete,
}: {
  meetings: readonly TrashedMeeting[];
  onDelete: (id: MeetingId) => void;
}) {
  if (meetings.length === 0) {
    return (
      <EmptyState
        className="pt-[14vh]"
        icon={Trash2}
        title={copy.trash.emptyTitle}
        description={copy.trash.emptyDescription}
      />
    );
  }
  return (
    <ul className="flex flex-col">
      {meetings.map((meeting) => (
        <TrashRow key={meeting.id} meeting={meeting} onDelete={onDelete} />
      ))}
    </ul>
  );
}

/** The Trash page. */
export function TrashScreen() {
  const trash = useTrash();
  const [deleting, setDeleting] = useState<MeetingId[]>([]);
  return (
    <Page>
      <PageHeader title={copy.trash.title} description={copy.trash.description} className="pb-4" />
      {trash.data === undefined ? (
        <div role="status" aria-label="Loading">
          <Skeleton className="h-10 w-full" />
        </div>
      ) : (
        <TrashList
          meetings={trash.data}
          onDelete={(id) => {
            setDeleting([id]);
          }}
        />
      )}
      <ConfirmDialog
        open={deleting.length > 0}
        title={copy.trash.confirmTitle(deleting.length)}
        description={copy.trash.confirmDescription}
        confirm={copy.trash.deleteForever}
        cancel={copy.trash.cancel}
        tone="danger-solid"
        onConfirm={() => deleteForever(deleting)}
        onClose={() => {
          setDeleting([]);
        }}
      />
    </Page>
  );
}
