/**
 * "Add to folder" for a meeting row's ⋯ menu (the keyboard path for
 * dragging a meeting onto a folder): a submenu of every folder, checked
 * where the meeting already sits, and "New folder…". Choosing a folder
 * files or unfiles the meeting.
 */
import { useState } from 'react';

import { Check, FolderPlus, ICON_PROPS } from '@/ui/icons';
import {
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
} from '@/ui/menu';

import { foldersCopy } from '../copy';
import { folderIcon } from '../folder-icons';
import { useFileMeeting } from '../hooks/use-file-meeting';
import { useFolderTree, useMeetingFolders } from '../queries';
import type { Folder, MeetingId } from '../types';
import { FolderNameDialog } from './folder-name-dialog';

/** Indentation per tree level inside the submenu, in px. */
const INDENT_PX = 12;

/** The folder list, mounted only while the submenu is open. */
function FolderItems({ meetingId }: { meetingId: MeetingId }) {
  const entries = useFolderTree().data?.entries ?? [];
  const inFolders = new Set((useMeetingFolders(meetingId).data ?? []).map((folder) => folder.id));
  const { file, unfile } = useFileMeeting();
  return entries.map(({ folder, depth }) => {
    const ItemIcon = folderIcon(folder.icon);
    const filed = inFolders.has(folder.id);
    return (
      <DropdownMenuItem
        key={folder.id}
        style={{ paddingLeft: 8 + depth * INDENT_PX }}
        onSelect={() => {
          if (filed) unfile(folder, meetingId);
          else file(folder, meetingId);
        }}
      >
        <ItemIcon {...ICON_PROPS} />
        <span className="min-w-0 flex-1 truncate">{folder.name}</span>
        {filed ? <Check {...ICON_PROPS} className="text-fg" /> : null}
      </DropdownMenuItem>
    );
  });
}

/** Props of {@link AddToFolderSubmenu}. */
export interface AddToFolderSubmenuProps {
  meetingId: MeetingId;
  /** Opens the new-folder dialog (it outlives the menu, so the row owns it). */
  onNewFolder: () => void;
}

/** The "Add to folder" submenu. */
export function AddToFolderSubmenu({ meetingId, onNewFolder }: AddToFolderSubmenuProps) {
  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger>
        <FolderPlus {...ICON_PROPS} />
        {foldersCopy.addToFolder}
      </DropdownMenuSubTrigger>
      <DropdownMenuSubContent>
        <FolderItems meetingId={meetingId} />
        <DropdownMenuSeparator />
        <DropdownMenuItem onSelect={onNewFolder}>
          <FolderPlus {...ICON_PROPS} />
          {foldersCopy.newFolder}…
        </DropdownMenuItem>
      </DropdownMenuSubContent>
    </DropdownMenuSub>
  );
}

/** A new-folder dialog that files the meeting into the folder it creates. */
export function useNewFolderFor(meetingId: MeetingId) {
  const [open, setOpen] = useState(false);
  const { file } = useFileMeeting();
  const dialog = (
    <FolderNameDialog
      target={open ? { kind: 'create', parent: null } : null}
      onClose={() => {
        setOpen(false);
      }}
      onCreated={(folder: Folder) => {
        file(folder, meetingId);
      }}
    />
  );
  return {
    open: () => {
      setOpen(true);
    },
    dialog,
  };
}
