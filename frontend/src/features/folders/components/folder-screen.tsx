/**
 * A folder's page (`/folders/$folderId`, ORG-05): its header, description
 * and auto-add rules, its subfolders and its meetings. Folder chat (AI-02)
 * and integrations join it when they ship (UX-27: nothing unfinished shows).
 */
import { Link } from '@tanstack/react-router';

import { buttonVariants } from '@/ui/button';
import { EmptyState } from '@/ui/empty-state';
import { Folder as FolderGlyph, ICON_PROPS, NotebookText, X } from '@/ui/icons';
import { Page } from '@/ui/page-header';
import { Skeleton } from '@/ui/skeleton';
import { Hint } from '@/ui/tooltip';

import { foldersCopy } from '../copy';
import { folderIcon } from '../folder-icons';
import { useFileMeeting } from '../hooks/use-file-meeting';
import { useFolder, useFolderMeetings, useFolderTree } from '../queries';
import type { Folder, FolderId, FolderTreeEntry, Meeting } from '../types';
import { FolderDescription } from './folder-description';
import { FolderHeader } from './folder-header';
import { FolderMeetingList, MeetingListSkeleton } from './folder-meeting-list';
import { FolderRules } from './folder-rules';

/** The folders directly inside this one, as compact links. */
function Subfolders({ folder, entries }: { folder: Folder; entries: readonly FolderTreeEntry[] }) {
  const children = entries.filter((entry) => entry.folder.parentId === folder.id);
  if (children.length === 0) return null;
  return (
    <section aria-label={foldersCopy.subfolders} className="flex flex-wrap gap-1.5">
      {children.map(({ folder: child, meetingCount }) => {
        const ChildIcon = folderIcon(child.icon);
        return (
          <Link
            key={child.id}
            to="/folders/$folderId"
            params={{ folderId: child.id }}
            className="inline-flex h-8 items-center gap-2 rounded-md border border-border px-2.5 text-sm text-fg transition-colors duration-(--dur-fast) hover:bg-bg-muted"
          >
            <ChildIcon {...ICON_PROPS} className="text-fg-muted" />
            {child.name}
            <span className="text-xs text-fg-subtle tabular-nums">{meetingCount}</span>
          </Link>
        );
      })}
    </section>
  );
}

/** The folder's meetings, each removable from it. */
function Meetings({ folder }: { folder: Folder }) {
  const meetings = useFolderMeetings(folder.id);
  const { unfile } = useFileMeeting();
  if (meetings.isPending) return <MeetingListSkeleton />;
  if ((meetings.data ?? []).length === 0) {
    return (
      <EmptyState
        className="pt-[6vh]"
        icon={NotebookText}
        title={foldersCopy.emptyFolderTitle}
        description={foldersCopy.emptyFolderDescription}
        action={
          <Link to="/meetings" className={buttonVariants({ variant: 'secondary' })}>
            {foldersCopy.openMeetings}
          </Link>
        }
      />
    );
  }
  const removeAction = (meeting: Meeting) => (
    <Hint label={foldersCopy.removeFromFolder}>
      <button
        type="button"
        aria-label={`${foldersCopy.removeFromFolder}: ${meeting.title}`}
        onClick={() => {
          unfile(folder, meeting.id);
        }}
        className="flex size-7 items-center justify-center rounded-sm text-fg-muted opacity-0 transition-opacity duration-(--dur-fast) group-hover:opacity-100 hover:bg-bg hover:text-fg focus-visible:opacity-100"
      >
        <X {...ICON_PROPS} />
      </button>
    </Hint>
  );
  return <FolderMeetingList meetings={meetings.data ?? []} action={removeAction} />;
}

function Loading() {
  return (
    <div aria-busy className="flex flex-col gap-3">
      <Skeleton className="h-8 w-2/5" />
      <Skeleton className="h-3 w-3/5" />
    </div>
  );
}

/** The folder page. */
export function FolderScreen({ folderId }: { folderId: FolderId }) {
  const folder = useFolder(folderId);
  const entries = useFolderTree().data?.entries ?? [];
  if (folder.isError) return <EmptyState icon={FolderGlyph} title={foldersCopy.notFound} />;
  return (
    <Page>
      {folder.data === undefined ? (
        <Loading />
      ) : (
        <div className="flex min-h-full flex-col gap-6">
          <div className="flex flex-col gap-1">
            <FolderHeader folder={folder.data} entries={entries} />
            <FolderDescription
              key={`${folder.data.id}:${folder.data.description}`}
              folder={folder.data}
            />
            <FolderRules folderId={folderId} />
          </div>
          <Subfolders folder={folder.data} entries={entries} />
          <section aria-labelledby="folder-meetings" className="flex-1">
            <h2
              id="folder-meetings"
              className="flex h-8 items-center px-3 text-xs font-semibold text-fg-muted"
            >
              {foldersCopy.meetings}
            </h2>
            <Meetings folder={folder.data} />
          </section>
        </div>
      )}
    </Page>
  );
}
