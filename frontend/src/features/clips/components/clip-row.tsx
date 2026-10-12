/**
 * One clip in the Clips tab: Play (the range in the meeting's player), its
 * name (click to rename in place), its range, and ⋯ with Copy link, Show
 * in Finder and Delete.
 */
import { type KeyboardEvent, useState } from 'react';

import { playerActions } from '@/features/player';
import {
  Ellipsis,
  FolderOpen,
  type Icon,
  ICON_PROPS,
  Link,
  Pencil,
  Play,
  Trash2,
  Video,
} from '@/ui/icons';
import { IconButton } from '@/ui/icon-button';
import { Input } from '@/ui/input';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/ui/menu';

import { clipsCopy } from '../copy';
import { copyMomentLink } from '../moment-link';
import { useDeleteClip, useRenameClip, useRevealClip } from '../queries';
import type { Clip } from '../types';
import { clipRange } from '../time';

/** The name, or a field while it is being renamed. */
function ClipName({ clip, editing, onDone }: { clip: Clip; editing: boolean; onDone: () => void }) {
  const rename = useRenameClip();
  const [name, setName] = useState(clip.name);
  if (!editing) return <span className="truncate text-sm text-fg">{clip.name}</span>;
  const commit = () => {
    if (name.trim() !== '' && name !== clip.name) rename.mutate({ id: clip.id, name });
    onDone();
  };
  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'Enter') commit();
    if (event.key === 'Escape') onDone();
  };
  return (
    <Input
      autoFocus
      aria-label={clipsCopy.renameLabel}
      value={name}
      onChange={(event) => {
        setName(event.target.value);
      }}
      onBlur={commit}
      onKeyDown={onKeyDown}
      className="h-7"
    />
  );
}

/** One entry of the ⋯ menu. */
interface MenuEntry {
  label: string;
  icon: Icon;
  run: () => void;
  destructive?: boolean;
}

/** The ⋯ menu's entries for `clip`. */
function useMenuEntries(clip: Clip, onRename: () => void): MenuEntry[] {
  const reveal = useRevealClip();
  const remove = useDeleteClip();
  return [
    { label: clipsCopy.rename, icon: Pencil, run: onRename },
    {
      label: clipsCopy.copyLink,
      icon: Link,
      run: () => void copyMomentLink(clip.meetingId, clip.start),
    },
    {
      label: clipsCopy.reveal,
      icon: FolderOpen,
      run: () => {
        reveal.mutate(clip.id);
      },
    },
    {
      label: clipsCopy.delete,
      icon: Trash2,
      destructive: true,
      run: () => {
        remove.mutate(clip.id);
      },
    },
  ];
}

/** ⋯: rename, copy link, Finder, delete. */
function ClipMenu({ clip, onRename }: { clip: Clip; onRename: () => void }) {
  const entries = useMenuEntries(clip, onRename);
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        aria-label={clipsCopy.more}
        className="flex size-7 items-center justify-center rounded-sm text-fg-muted hover:bg-bg-muted hover:text-fg"
      >
        <Ellipsis {...ICON_PROPS} />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="min-w-[180px]">
        {entries.map(({ label, icon: EntryIcon, run, destructive }) => (
          <DropdownMenuItem key={label} destructive={destructive === true} onSelect={run}>
            <EntryIcon {...ICON_PROPS} />
            {label}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** One row. */
export function ClipRow({ clip }: { clip: Clip }) {
  const [editing, setEditing] = useState(false);
  const range = clipRange(clip.start, clip.end);
  return (
    <li className="group flex items-center gap-3 rounded-md px-2 py-2 hover:bg-bg-subtle">
      <IconButton
        icon={Play}
        label={clipsCopy.play}
        onClick={() => {
          playerActions.playRange(clip.meetingId, clip.start, clip.end);
        }}
      />
      <div className="flex min-w-0 flex-1 flex-col">
        <ClipName
          key={String(editing)}
          clip={clip}
          editing={editing}
          onDone={() => {
            setEditing(false);
          }}
        />
        <span className="flex items-center gap-1.5 text-xs text-fg-muted tabular-nums">
          {clip.media === 'video' ? <Video {...ICON_PROPS} size={12} /> : null}
          {range}
        </span>
      </div>
      <ClipMenu
        clip={clip}
        onRename={() => {
          setEditing(true);
        }}
      />
    </li>
  );
}
