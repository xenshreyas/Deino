/**
 * One 40px row of the Meetings list (ui.md §3 List row; Circleback §14:
 * invitee avatars and colour-coded tags): a selection checkbox, the title,
 * up to two tags, up to three avatars, the length and the time, and a ⋯
 * menu (Add to folder, Delete) revealed on hover or focus. The row drags
 * onto a sidebar folder.
 */
import { Link } from '@tanstack/react-router';

import { EntityAvatar } from '@/features/entities';
import { AddToFolderSubmenu, meetingDragSource, useNewFolderFor } from '@/features/folders';
import { TagDot } from '@/features/tags';
import { cn } from '@/ui/cn';
import { Ellipsis, ICON_PROPS, Trash2 } from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/ui/menu';

import { trashMeetings } from '../actions';
import { copy } from '../copy';
import { rowDuration, rowTime } from '../day-groups';
import type { ListedAttendee, ListedMeeting, ListedTag } from '../types';
import { RowCheckbox } from './row-checkbox';

/** Tags shown before the rest fold into "+n". */
const SHOWN_TAGS = 2;
/** Avatars shown before the rest fold into "+n". */
const SHOWN_PEOPLE = 3;

/** Props of {@link MeetingListRow}. */
export interface MeetingListRowProps {
  meeting: ListedMeeting;
  selected: boolean;
  /** Something is selected, so every checkbox shows. */
  selecting: boolean;
  /** A click on the checkbox; `range` when Shift was held. */
  onSelect: (range: boolean) => void;
}

/** Up to two tags, then "+n". */
function RowTags({ tags }: { tags: readonly ListedTag[] }) {
  if (tags.length === 0) return null;
  const rest = tags.length - SHOWN_TAGS;
  return (
    <span className="hidden min-w-0 shrink items-center gap-3 md:flex">
      {tags.slice(0, SHOWN_TAGS).map((tag) => (
        <span
          key={tag.id}
          className="inline-flex min-w-0 items-center gap-1.5 text-xs text-fg-muted"
        >
          <TagDot color={tag.color} />
          <span className="truncate">{tag.name}</span>
        </span>
      ))}
      {rest > 0 ? <span className="text-xs text-fg-subtle tabular-nums">+{rest}</span> : null}
    </span>
  );
}

/** Up to three overlapping avatars, the user's left out, then "+n". */
function RowPeople({ people }: { people: readonly ListedAttendee[] }) {
  const others = people.filter((person) => !person.isMe);
  if (others.length === 0) return null;
  const rest = others.length - SHOWN_PEOPLE;
  return (
    <span
      className="hidden shrink-0 items-center sm:flex"
      aria-label={others.map((p) => p.name).join(', ')}
    >
      {others.slice(0, SHOWN_PEOPLE).map((person) => (
        <EntityAvatar
          key={person.personId}
          id={person.personId}
          name={person.name}
          kind="person"
          size={20}
          className="-ml-1 ring-2 ring-bg first:ml-0"
        />
      ))}
      {rest > 0 ? (
        <span className="ml-1 text-xs text-fg-subtle tabular-nums">{copy.moreAttendees(rest)}</span>
      ) : null}
    </span>
  );
}

/** The row's ⋯ menu: Add to folder, Delete. */
function RowMenu({ meeting }: { meeting: ListedMeeting }) {
  const newFolder = useNewFolderFor(meeting.id);
  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger
          aria-label={`${meeting.title} actions`}
          className={cn(
            'absolute right-1.5 flex size-7 items-center justify-center rounded-sm text-fg-muted',
            'opacity-0 transition-opacity duration-(--dur-fast) group-hover:opacity-100 hover:bg-bg focus-visible:opacity-100 data-[state=open]:opacity-100',
          )}
        >
          <Ellipsis {...ICON_PROPS} />
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <AddToFolderSubmenu meetingId={meeting.id} onNewFolder={newFolder.open} />
          <DropdownMenuSeparator />
          <DropdownMenuItem destructive onSelect={() => void trashMeetings([meeting.id])}>
            <Trash2 {...ICON_PROPS} />
            {copy.bulk.delete}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
      {newFolder.dialog}
    </>
  );
}

/** One row. */
export function MeetingListRow({ meeting, selected, selecting, onSelect }: MeetingListRowProps) {
  return (
    <li
      {...meetingDragSource(meeting)}
      aria-selected={selected}
      className={cn(
        'group relative flex h-10 items-center rounded-md transition-colors duration-(--dur-fast) hover:bg-bg-muted has-[a:focus-visible]:bg-bg-muted',
        selected && 'bg-accent-soft hover:bg-accent-soft',
      )}
    >
      <RowCheckbox
        checked={selected}
        label={copy.select(meeting.title)}
        visible={selecting}
        onToggle={onSelect}
      />
      <Link
        to="/meetings/$meetingId"
        params={{ meetingId: meeting.id }}
        draggable={false}
        className="flex h-full min-w-0 flex-1 items-center gap-3 rounded-md pr-10 pl-1 focus-visible:outline-offset-[-2px]"
      >
        <span className="min-w-0 flex-1 truncate text-sm text-fg">{meeting.title}</span>
        <RowTags tags={meeting.tags} />
        <RowPeople people={meeting.attendees} />
        <span className="w-12 shrink-0 text-right text-xs text-fg-subtle tabular-nums">
          {rowDuration(meeting.duration)}
        </span>
        <span className="w-10 shrink-0 text-right text-xs text-fg-muted tabular-nums">
          {rowTime(meeting.createdAt)}
        </span>
      </Link>
      <RowMenu meeting={meeting} />
    </li>
  );
}
