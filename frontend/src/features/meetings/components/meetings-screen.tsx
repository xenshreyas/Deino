/**
 * The Meetings page: every meeting under sticky date headers, with its tags,
 * a tag filter, and rename, delete and "Add to folder" on each row. Rows
 * drag onto the sidebar's folders. ORG-07 grows it into views, more filters
 * and bulk actions (ui.md §4.4).
 */
import { Link } from '@tanstack/react-router';
import { useState } from 'react';

import { TagFilter, type TagId } from '@/features/tags';
import { Alert, AlertDescription } from '@/ui/alert';
import { Button, buttonVariants } from '@/ui/button';
import { EmptyState } from '@/ui/empty-state';
import { CircleAlert, ICON_PROPS, Mic, NotebookText, Tag } from '@/ui/icons';
import { Kbd } from '@/ui/kbd';
import { Page, PageHeader } from '@/ui/page-header';
import { Skeleton } from '@/ui/skeleton';

import { meetingsCopy } from '../copy';
import { bucketMeetings, meetingTime } from '../date-buckets';
import { useMeetings } from '../queries';
import { filterByTags, type TagIndex, useTagIndex } from '../tag-index';
import type { Meeting } from '../types';
import { DeleteMeetingDialog, RenameMeetingDialog } from './meeting-dialogs';
import { MeetingRow } from './meeting-row';

/** Props of {@link MeetingsScreen}; the route wires in the shell's actions and the URL filter. */
export interface MeetingsScreenProps {
  onStartRecording: () => void;
  shortcuts: { newNote: string; record: string };
  /** Show only meetings carrying any of these tags; empty shows all. */
  tagFilter: readonly TagId[];
  onTagFilterChange: (tags: TagId[]) => void;
}

/** Five row-shaped placeholders while the list loads. */
function ListSkeleton() {
  return (
    <div role="status" aria-label="Loading" className="flex flex-col gap-1 pt-2">
      {Array.from({ length: 5 }, (_, index) => (
        <div key={index} className="flex h-10 items-center gap-3 px-3">
          <Skeleton className="size-4 rounded-full" />
          <Skeleton className="h-2.5 flex-1" />
          <Skeleton className="h-2.5 w-10" />
        </div>
      ))}
    </div>
  );
}

/** The grouped list with its two dialogs. */
function MeetingList({ meetings, tags }: { meetings: readonly Meeting[]; tags: TagIndex }) {
  const [renaming, setRenaming] = useState<Meeting | null>(null);
  const [deleting, setDeleting] = useState<Meeting | null>(null);
  const now = new Date();
  return (
    <>
      {bucketMeetings(meetings, now).map((bucket) => (
        <section key={bucket.label} aria-label={bucket.label}>
          <h2 className="sticky top-0 z-(--z-sticky) flex h-8 items-center bg-bg px-3 text-xs font-semibold text-fg-muted">
            {bucket.label}
          </h2>
          <ul className="flex flex-col">
            {bucket.meetings.map((meeting) => (
              <MeetingRow
                key={meeting.id}
                meeting={meeting}
                tags={tags.get(meeting.id) ?? []}
                time={meetingTime(meeting.createdAt, now)}
                onRename={setRenaming}
                onDelete={setDeleting}
              />
            ))}
          </ul>
        </section>
      ))}
      <RenameMeetingDialog
        meeting={renaming}
        onClose={() => {
          setRenaming(null);
        }}
      />
      <DeleteMeetingDialog
        meeting={deleting}
        onClose={() => {
          setDeleting(null);
        }}
      />
    </>
  );
}

/** Nothing carries the chosen tags. */
function NoMatches({ onClear }: { onClear: () => void }) {
  return (
    <EmptyState
      className="pt-[10vh]"
      icon={Tag}
      title={meetingsCopy.noTaggedTitle}
      description={meetingsCopy.noTaggedDescription}
      action={
        <Button variant="secondary" onClick={onClear}>
          {meetingsCopy.clearFilter}
        </Button>
      }
    />
  );
}

/** The recording call to action when there are no meetings at all. */
function NoMeetings({
  onStartRecording,
  shortcuts,
}: Pick<MeetingsScreenProps, 'onStartRecording' | 'shortcuts'>) {
  return (
    <EmptyState
      className="pt-[14vh]"
      icon={NotebookText}
      title={meetingsCopy.emptyTitle}
      description={meetingsCopy.emptyDescription}
      action={
        <Button variant="secondary" onClick={onStartRecording}>
          <Mic {...ICON_PROPS} />
          {meetingsCopy.startRecording}
          <Kbd className="ml-1">{shortcuts.record}</Kbd>
        </Button>
      }
    />
  );
}

/** Loading, error, empty or the (filtered) list. */
function MeetingsBody(props: MeetingsScreenProps) {
  const meetings = useMeetings();
  const tags = useTagIndex();
  const filtering = props.tagFilter.length > 0;
  if (meetings.isPending || (filtering && !tags.ready)) return <ListSkeleton />;
  if (meetings.isError) {
    return (
      <Alert variant="destructive">
        <CircleAlert {...ICON_PROPS} />
        <AlertDescription className="flex items-center justify-between gap-3">
          {meetingsCopy.loadError}
          <Button variant="ghost" size="sm" onClick={() => void meetings.refetch()}>
            {meetingsCopy.retry}
          </Button>
        </AlertDescription>
      </Alert>
    );
  }
  if (meetings.data.length === 0) return <NoMeetings {...props} />;
  const shown = filterByTags(meetings.data, tags.index, props.tagFilter);
  if (shown.length === 0)
    return (
      <NoMatches
        onClear={() => {
          props.onTagFilterChange([]);
        }}
      />
    );
  return <MeetingList meetings={shown} tags={tags.index} />;
}

/** The Meetings page. */
export function MeetingsScreen(props: MeetingsScreenProps) {
  return (
    <Page>
      <PageHeader
        title={meetingsCopy.title}
        className="pb-4"
        actions={
          <Link to="/" className={buttonVariants({ variant: 'secondary' })}>
            {meetingsCopy.newNote}
            <Kbd className="ml-1">{props.shortcuts.newNote}</Kbd>
          </Link>
        }
      />
      <div className="flex h-8 items-center gap-4 pb-4">
        <TagFilter selected={props.tagFilter} onChange={props.onTagFilterChange} />
      </div>
      <MeetingsBody {...props} />
    </Page>
  );
}
