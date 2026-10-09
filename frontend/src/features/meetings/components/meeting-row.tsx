import { Link } from '@tanstack/react-router';

import { AddToFolderSubmenu, meetingDragSource, useNewFolderFor } from '@/features/folders';
import { type Tag, TagMark } from '@/features/tags';
import { cn } from '@/ui/cn';
import { Ellipsis, ICON_PROPS, NotebookText, Pencil, Trash2 } from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/ui/menu';

import { meetingsCopy } from '../copy';
import type { Meeting } from '../types';

/** Tags shown in a row before the rest fold into "+n". */
const SHOWN_TAGS = 2;

/** Props of {@link MeetingRow}. */
export interface MeetingRowProps {
  meeting: Meeting;
  /** The meeting's tags, by name. */
  tags: readonly Tag[];
  /** The trailing time, already formatted. */
  time: string;
  onRename: (meeting: Meeting) => void;
  onDelete: (meeting: Meeting) => void;
}

/** The row's ⋯ menu: Rename, Add to folder, Delete. */
function RowMenu({ meeting, onRename, onDelete }: Omit<MeetingRowProps, 'time' | 'tags'>) {
  const newFolder = useNewFolderFor(meeting.id);
  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger
          aria-label={meetingsCopy.rowActions(meeting.title)}
          className={cn(
            'absolute right-1.5 flex size-7 items-center justify-center rounded-sm text-fg-muted',
            'opacity-0 transition-opacity duration-(--dur-fast) group-hover:opacity-100 hover:bg-bg focus-visible:opacity-100 data-[state=open]:opacity-100',
          )}
        >
          <Ellipsis {...ICON_PROPS} />
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem
            onSelect={() => {
              onRename(meeting);
            }}
          >
            <Pencil {...ICON_PROPS} />
            {meetingsCopy.rename}
          </DropdownMenuItem>
          <AddToFolderSubmenu meetingId={meeting.id} onNewFolder={newFolder.open} />
          <DropdownMenuSeparator />
          <DropdownMenuItem
            destructive
            onSelect={() => {
              onDelete(meeting);
            }}
          >
            <Trash2 {...ICON_PROPS} />
            {meetingsCopy.delete}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
      {newFolder.dialog}
    </>
  );
}

/** Up to two tag marks, then "+n" (ui.md §3 Tag pill: dense rows). */
function RowTags({ tags }: { tags: readonly Tag[] }) {
  if (tags.length === 0) return null;
  const rest = tags.length - SHOWN_TAGS;
  return (
    <span className="hidden min-w-0 shrink items-center gap-3 sm:flex">
      {tags.slice(0, SHOWN_TAGS).map((tag) => (
        <TagMark key={tag.id} tag={tag} />
      ))}
      {rest > 0 ? <span className="text-xs text-fg-subtle tabular-nums">+{rest}</span> : null}
    </span>
  );
}

/**
 * One 40px list row (ui.md §3 List row): icon, title, tags, trailing time,
 * and a ⋯ menu revealed on hover or focus. The row drags onto a sidebar folder.
 */
export function MeetingRow({ meeting, tags, time, onRename, onDelete }: MeetingRowProps) {
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
        <RowTags tags={tags} />
        <span className="shrink-0 text-xs text-fg-muted tabular-nums">{time}</span>
      </Link>
      <RowMenu meeting={meeting} onRename={onRename} onDelete={onDelete} />
    </li>
  );
}
