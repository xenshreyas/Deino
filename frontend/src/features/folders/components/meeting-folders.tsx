/**
 * A meeting's folders in its header (ui.md §3 Meeting header: folder chip):
 * a chip per folder that opens it, then a ghost "Add to folder" chip with a
 * searchable folder list where choosing toggles the meeting in a folder.
 */
import { Link } from '@tanstack/react-router';
import { useState } from 'react';

import { Chip } from '@/ui/chip';
import { CommandItem } from '@/ui/command';
import { FolderPlus, ICON_PROPS } from '@/ui/icons';
import { Popover, PopoverContent, PopoverTrigger } from '@/ui/popover';

import { foldersCopy } from '../copy';
import { folderIcon } from '../folder-icons';
import { useFileMeeting } from '../hooks/use-file-meeting';
import { useFolderTree, useMeetingFolders } from '../queries';
import type { Folder, MeetingId } from '../types';
import { useNewFolderFor } from './add-to-folder';
import { FolderPicker } from './folder-picker';

/** Props of {@link MeetingFolders}. */
export interface MeetingFoldersProps {
  meetingId: MeetingId;
  /** Classes for the ghost "Add to folder" chip (the header reveals it on hover). */
  addChipClassName?: string | undefined;
}

/** A folder the meeting sits in, as a link chip. */
function FolderChip({ folder }: { folder: Folder }) {
  const FolderIcon = folderIcon(folder.icon);
  return (
    <Link
      to="/folders/$folderId"
      params={{ folderId: folder.id }}
      className="inline-flex h-6 items-center gap-1.5 rounded-full border border-border px-2 text-xs whitespace-nowrap text-fg-muted transition-colors duration-(--dur-fast) hover:bg-bg-muted hover:text-fg"
    >
      <FolderIcon {...ICON_PROPS} size={14} />
      <span className="max-w-40 truncate">{folder.name}</span>
    </Link>
  );
}

/** Every folder, checked where the meeting sits; choosing one toggles it. */
function FolderToggles({
  meetingId,
  filed,
  onNewFolder,
}: {
  meetingId: MeetingId;
  filed: readonly Folder[];
  onNewFolder: () => void;
}) {
  const entries = useFolderTree().data?.entries ?? [];
  const { file, unfile } = useFileMeeting();
  const selected = new Set(filed.map((folder) => folder.id));
  const toggle = (folder: Folder) => {
    if (selected.has(folder.id)) unfile(folder, meetingId);
    else file(folder, meetingId);
  };
  return (
    <FolderPicker
      entries={entries}
      selected={selected}
      empty={foldersCopy.emptyTreeTitle}
      onSelect={({ folder }) => {
        toggle(folder);
      }}
    >
      <CommandItem value="new folder" onSelect={onNewFolder}>
        <FolderPlus {...ICON_PROPS} />
        {foldersCopy.newFolder}…
      </CommandItem>
    </FolderPicker>
  );
}

/** The "Add to folder" chip and its folder list. */
function AddChip({
  meetingId,
  filed,
  className,
}: {
  meetingId: MeetingId;
  filed: readonly Folder[];
  className?: string | undefined;
}) {
  const [open, setOpen] = useState(false);
  const newFolder = useNewFolderFor(meetingId);
  const startNewFolder = () => {
    setOpen(false);
    newFolder.open();
  };
  return (
    <>
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <Chip variant="ghost" icon={FolderPlus} className={className}>
            {foldersCopy.addToFolder}
          </Chip>
        </PopoverTrigger>
        <PopoverContent align="start" className="w-64 p-0">
          <FolderToggles meetingId={meetingId} filed={filed} onNewFolder={startNewFolder} />
        </PopoverContent>
      </Popover>
      {newFolder.dialog}
    </>
  );
}

/** The meeting's folder chips and the "Add to folder" chip. */
export function MeetingFolders({ meetingId, addChipClassName }: MeetingFoldersProps) {
  const folders = useMeetingFolders(meetingId).data ?? [];
  return (
    <>
      {folders.map((folder) => (
        <FolderChip key={folder.id} folder={folder} />
      ))}
      <AddChip meetingId={meetingId} filed={folders} className={addChipClassName} />
    </>
  );
}
