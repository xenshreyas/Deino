/**
 * The Folders overview (`/folders`: every folder as an indented list, plus
 * Unfiled) and the Unfiled smart folder (`/folders/unfiled`: meetings in no
 * folder, each with "Add to folder").
 */
import { Link } from '@tanstack/react-router';
import { useState } from 'react';

import { Button } from '@/ui/button';
import { EmptyState } from '@/ui/empty-state';
import { Ellipsis, Folder, ICON_PROPS, Inbox, Plus } from '@/ui/icons';
import { DropdownMenu, DropdownMenuContent, DropdownMenuTrigger } from '@/ui/menu';
import { Page, PageHeader } from '@/ui/page-header';

import { foldersCopy } from '../copy';
import { folderIcon } from '../folder-icons';
import { useFolderTree, useUnfiledMeetings } from '../queries';
import type { FolderTree, FolderTreeEntry, Meeting } from '../types';
import { AddToFolderSubmenu, useNewFolderFor } from './add-to-folder';
import { FolderMeetingList, MeetingListSkeleton } from './folder-meeting-list';
import { FolderNameDialog, type FolderNameTarget } from './folder-name-dialog';

/** Indentation per tree level, in px. */
const INDENT_PX = 20;

const ROW =
  'flex h-10 items-center gap-3 rounded-md pr-3 text-sm transition-colors duration-(--dur-fast) hover:bg-bg-muted focus-visible:outline-offset-[-2px]';

function OverviewRow({ entry }: { entry: FolderTreeEntry }) {
  const RowIcon = folderIcon(entry.folder.icon);
  return (
    <li>
      <Link
        to="/folders/$folderId"
        params={{ folderId: entry.folder.id }}
        className={ROW}
        style={{ paddingLeft: 12 + entry.depth * INDENT_PX }}
      >
        <RowIcon {...ICON_PROPS} className="shrink-0 text-fg-muted" />
        <span className="min-w-0 flex-1 truncate text-fg">{entry.folder.name}</span>
        <span className="text-xs text-fg-muted tabular-nums">
          {foldersCopy.meetingCount(entry.meetingCount)}
        </span>
      </Link>
    </li>
  );
}

/** Every folder, indented by depth, then Unfiled. */
function OverviewList({ tree }: { tree: FolderTree }) {
  return (
    <ul className="flex flex-col">
      {tree.entries.map((entry) => (
        <OverviewRow key={entry.folder.id} entry={entry} />
      ))}
      <li>
        <Link to="/folders/unfiled" className={`${ROW} pl-3`}>
          <Inbox {...ICON_PROPS} className="shrink-0 text-fg-muted" />
          <span className="min-w-0 flex-1 truncate text-fg">{foldersCopy.unfiled}</span>
          <span className="text-xs text-fg-muted tabular-nums">
            {foldersCopy.meetingCount(tree.unfiledCount)}
          </span>
        </Link>
      </li>
    </ul>
  );
}

function NewFolderButton({ onClick }: { onClick: () => void }) {
  return (
    <Button variant="secondary" onClick={onClick}>
      <Plus {...ICON_PROPS} />
      {foldersCopy.newFolder}
    </Button>
  );
}

/** `/folders`: every folder, then Unfiled; an empty state before the first folder. */
export function FoldersScreen() {
  const tree = useFolderTree();
  const [naming, setNaming] = useState<FolderNameTarget>(null);
  const openNew = () => {
    setNaming({ kind: 'create', parent: null });
  };
  const empty = tree.data?.entries.length === 0;
  return (
    <Page>
      <PageHeader
        title={foldersCopy.folders}
        description={foldersCopy.overviewDescription}
        actions={<NewFolderButton onClick={openNew} />}
      />
      {empty ? (
        <EmptyState
          className="pt-[10vh]"
          icon={Folder}
          title={foldersCopy.emptyTreeTitle}
          description={foldersCopy.emptyTreeDescription}
          action={<NewFolderButton onClick={openNew} />}
        />
      ) : null}
      {tree.data === undefined || empty ? null : <OverviewList tree={tree.data} />}
      <FolderNameDialog
        target={naming}
        onClose={() => {
          setNaming(null);
        }}
      />
    </Page>
  );
}

/** An unfiled meeting's ⋯ menu: "Add to folder". */
function UnfiledAction({ meeting }: { meeting: Meeting }) {
  const newFolder = useNewFolderFor(meeting.id);
  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger
          aria-label={`${foldersCopy.addToFolder}: ${meeting.title}`}
          className="flex size-7 items-center justify-center rounded-sm text-fg-muted opacity-0 transition-opacity duration-(--dur-fast) group-hover:opacity-100 hover:bg-bg focus-visible:opacity-100 data-[state=open]:opacity-100"
        >
          <Ellipsis {...ICON_PROPS} />
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <AddToFolderSubmenu meetingId={meeting.id} onNewFolder={newFolder.open} />
        </DropdownMenuContent>
      </DropdownMenu>
      {newFolder.dialog}
    </>
  );
}

/** `/folders/unfiled`: the meetings in no folder. */
export function UnfiledScreen() {
  const meetings = useUnfiledMeetings();
  return (
    <Page>
      <PageHeader title={foldersCopy.unfiled} description={foldersCopy.unfiledDescription} />
      {meetings.isPending ? (
        <MeetingListSkeleton />
      ) : (
        <FolderMeetingList
          meetings={meetings.data ?? []}
          action={(meeting) => <UnfiledAction meeting={meeting} />}
        />
      )}
    </Page>
  );
}
