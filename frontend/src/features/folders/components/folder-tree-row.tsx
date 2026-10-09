/**
 * One folder in the sidebar tree (ui.md §3 Sidebar and Nav item): 30px,
 * indented 12px a level, a 16px disclosure, the folder's icon and name, its
 * meeting count (the ⋯ menu takes its place on hover). The row is a drag
 * source for the folder and a drop target for meetings and folders.
 */
import { Link } from '@tanstack/react-router';
import type { KeyboardEvent } from 'react';

import { cn } from '@/ui/cn';
import { ChevronRight, ICON_PROPS } from '@/ui/icons';

import { foldersCopy } from '../copy';
import { dragSource } from '../dnd';
import { folderIcon } from '../folder-icons';
import { useFolderDrop } from '../hooks/use-folder-drop';
import type { FolderTreeEntry } from '../types';
import { FolderMenu } from './folder-menu';

/** Indentation per tree level, in px (ui.md §3 Sidebar). */
const INDENT_PX = 12;

/** Props of {@link FolderTreeRow}. */
export interface FolderTreeRowProps {
  entry: FolderTreeEntry;
  /** The whole tree, for drop rules and the move dialog. */
  entries: readonly FolderTreeEntry[];
  expandable: boolean;
  expanded: boolean;
  /** True for the row that takes Tab (roving focus). */
  tabbable: boolean;
  onToggle: () => void;
  onFocus: () => void;
  onKeyDown: (event: KeyboardEvent<HTMLAnchorElement>) => void;
}

/** The disclosure triangle, or a spacer for a folder without subfolders. */
function Disclosure({
  entry,
  expandable,
  expanded,
  onToggle,
}: Pick<FolderTreeRowProps, 'entry' | 'expandable' | 'expanded' | 'onToggle'>) {
  if (!expandable) return <span aria-hidden className="size-4 shrink-0" />;
  const name = entry.folder.name;
  return (
    <button
      type="button"
      tabIndex={-1}
      aria-label={expanded ? foldersCopy.collapse(name) : foldersCopy.expand(name)}
      onClick={onToggle}
      className="flex size-4 shrink-0 items-center justify-center rounded-xs text-fg-subtle hover:text-fg"
    >
      <ChevronRight
        {...ICON_PROPS}
        size={14}
        className={cn(
          'transition-transform duration-(--dur-fast) ease-out',
          expanded && 'rotate-90',
        )}
      />
    </button>
  );
}

/** The row: hover and active fills, and the accent drop-target ring (ui.md §3 Nav item). */
const ROW_CLASS = cn(
  'group/row relative flex h-[30px] items-center gap-1 rounded-sm pr-1 text-sm text-fg-muted',
  'transition-colors duration-(--dur-fast) ease-out hover:bg-bg-muted hover:text-fg',
  'has-[a[data-status=active]]:bg-bg-muted has-[a[data-status=active]]:text-fg',
  'data-over:bg-accent-soft data-over:text-fg data-over:ring-1 data-over:ring-accent data-over:ring-inset',
);

/** The folder's link, the treeitem that takes the focus and the arrow keys. */
function RowLink({
  entry,
  expandable,
  expanded,
  tabbable,
  onFocus,
  onKeyDown,
}: FolderTreeRowProps) {
  const { folder } = entry;
  const FolderIcon = folderIcon(folder.icon);
  return (
    <Link
      to="/folders/$folderId"
      params={{ folderId: folder.id }}
      role="treeitem"
      aria-level={entry.depth + 1}
      aria-expanded={expandable ? expanded : undefined}
      data-folder-id={folder.id}
      tabIndex={tabbable ? 0 : -1}
      onFocus={onFocus}
      onKeyDown={onKeyDown}
      draggable={false}
      className="flex h-full min-w-0 flex-1 items-center gap-2 rounded-sm pl-1 focus-visible:outline-offset-[-2px]"
    >
      <FolderIcon {...ICON_PROPS} className="shrink-0" />
      <span className="truncate">{folder.name}</span>
    </Link>
  );
}

/** One folder: drag source, drop target, link, count and ⋯ menu. */
export function FolderTreeRow(props: FolderTreeRowProps) {
  const { entry, entries } = props;
  const { folder } = entry;
  const drop = useFolderDrop(folder, entries);
  return (
    <li
      role="none"
      data-over={drop.isOver || undefined}
      {...dragSource({ kind: 'folder', id: folder.id }, folder.name)}
      {...drop.handlers}
      style={{ paddingLeft: 4 + entry.depth * INDENT_PX }}
      className={ROW_CLASS}
    >
      <Disclosure {...props} />
      <RowLink {...props} />
      <span className="shrink-0 pr-1 text-xs text-fg-subtle tabular-nums group-hover/row:hidden group-has-[[data-state=open]]/row:hidden">
        {entry.meetingCount === 0 ? null : entry.meetingCount}
      </span>
      <FolderMenu
        folder={folder}
        entries={entries}
        className="hidden group-hover/row:flex group-has-[:focus-visible]/row:flex data-[state=open]:flex"
      />
    </li>
  );
}
