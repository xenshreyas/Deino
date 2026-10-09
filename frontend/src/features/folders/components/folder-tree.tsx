/**
 * The sidebar's Folders entry (ui.md §3 Sidebar: "Folders" is a tree): the
 * nav row with a disclosure and a new-folder button, the folder tree with
 * arrow-key navigation, and the "Unfiled" smart folder. The shell renders
 * it in place of the plain Folders link (`nav.ts`).
 */
import { Link } from '@tanstack/react-router';
import { type KeyboardEvent, useState } from 'react';

import { cn } from '@/ui/cn';
import { ChevronRight, Folder, ICON_PROPS, Inbox, Plus } from '@/ui/icons';
import { Hint } from '@/ui/tooltip';

import { foldersCopy } from '../copy';
import { useFolderDrop } from '../hooks/use-folder-drop';
import { useFolderTree } from '../queries';
import { parentIds, visibleEntries } from '../tree';
import { treeKeyEffect } from '../tree-keys';
import { useFolderTreeStore } from '../tree-store';
import type { FolderId, FolderTreeEntry } from '../types';
import { FolderNameDialog, type FolderNameTarget } from './folder-name-dialog';
import { FolderTreeRow } from './folder-tree-row';

/** The nav row look shared with the shell's links (ui.md §3 Nav item). */
export const treeRowClassName = cn(
  'flex h-[30px] w-full items-center gap-2 rounded-sm px-2 text-sm text-fg-muted',
  'transition-colors duration-(--dur-fast) ease-out hover:bg-bg-muted hover:text-fg',
  'focus-visible:outline-offset-[-2px] data-[status=active]:bg-bg-muted data-[status=active]:text-fg',
);

/** A 24px ghost button on the heading row. */
const HEADING_BUTTON =
  'flex size-6 items-center justify-center rounded-sm text-fg-muted hover:bg-bg hover:text-fg';

/** "New folder" and the show/hide disclosure, revealed on hover or focus. */
function HeadingActions({ onNew }: { onNew: () => void }) {
  const open = useFolderTreeStore((state) => state.open);
  const toggleOpen = useFolderTreeStore((state) => state.toggleOpen);
  return (
    <div className="absolute right-1 flex items-center gap-0.5 opacity-0 transition-opacity duration-(--dur-fast) group-hover/heading:opacity-100 focus-within:opacity-100">
      <Hint label={foldersCopy.newFolder} side="right">
        <button
          type="button"
          aria-label={foldersCopy.newFolder}
          onClick={onNew}
          className={HEADING_BUTTON}
        >
          <Plus {...ICON_PROPS} />
        </button>
      </Hint>
      <button
        type="button"
        aria-label={foldersCopy.toggleTree}
        aria-expanded={open}
        onClick={toggleOpen}
        className={HEADING_BUTTON}
      >
        <ChevronRight
          {...ICON_PROPS}
          size={14}
          className={cn('transition-transform duration-(--dur-fast)', open && 'rotate-90')}
        />
      </button>
    </div>
  );
}

/**
 * The "Folders" row: a link to the overview and the heading actions.
 * Folders dropped here move to the top level.
 */
function FoldersHeading({
  entries,
  onNew,
}: {
  entries: readonly FolderTreeEntry[];
  onNew: () => void;
}) {
  const drop = useFolderDrop(null, entries);
  return (
    <div
      {...drop.handlers}
      data-over={drop.isOver || undefined}
      className="group/heading relative flex items-center rounded-sm data-over:bg-accent-soft data-over:ring-1 data-over:ring-accent data-over:ring-inset"
    >
      <Link to="/folders" activeOptions={{ exact: true }} className={cn(treeRowClassName, 'pr-14')}>
        <Folder {...ICON_PROPS} />
        <span className="truncate">{foldersCopy.folders}</span>
      </Link>
      <HeadingActions onNew={onNew} />
    </div>
  );
}

/** Focus moves and folder toggles from the arrow keys. */
function useTreeKeyboard(visible: readonly FolderTreeEntry[], entries: readonly FolderTreeEntry[]) {
  const collapsed = useFolderTreeStore((state) => state.collapsed);
  const toggleFolder = useFolderTreeStore((state) => state.toggleFolder);
  const [focused, setFocused] = useState<FolderId | null>(null);
  const parents = parentIds(entries);
  const onKeyDown = (id: FolderId) => (event: KeyboardEvent<HTMLAnchorElement>) => {
    const effect = treeKeyEffect(visible, { collapsed, parents }, id, event.key);
    if (effect === null) return;
    event.preventDefault();
    if (effect.toggle !== undefined) toggleFolder(effect.toggle);
    if (effect.focus === undefined) return;
    setFocused(effect.focus);
    const tree = event.currentTarget.closest('[role=tree]');
    tree?.querySelector<HTMLElement>(`[data-folder-id="${effect.focus}"]`)?.focus();
  };
  const tabbable = visible.some((entry) => entry.folder.id === focused)
    ? focused
    : (visible[0]?.folder.id ?? null);
  return { collapsed, parents, toggleFolder, onKeyDown, tabbable, setFocused };
}

/** The tree itself: one row per visible folder. */
function Tree({ entries }: { entries: readonly FolderTreeEntry[] }) {
  const collapsedSet = useFolderTreeStore((state) => state.collapsed);
  const visible = visibleEntries(entries, collapsedSet);
  const keys = useTreeKeyboard(visible, entries);
  return (
    <ul role="tree" aria-label={foldersCopy.folders} className="flex flex-col gap-px">
      {visible.map((entry) => {
        const id = entry.folder.id;
        return (
          <FolderTreeRow
            key={id}
            entry={entry}
            entries={entries}
            expandable={keys.parents.has(id)}
            expanded={!keys.collapsed.has(id)}
            tabbable={keys.tabbable === id}
            onToggle={() => {
              keys.toggleFolder(id);
            }}
            onFocus={() => {
              keys.setFocused(id);
            }}
            onKeyDown={keys.onKeyDown(id)}
          />
        );
      })}
    </ul>
  );
}

/** The Unfiled smart folder, last in the section. */
function UnfiledLink({ count }: { count: number }) {
  return (
    <Link to="/folders/unfiled" className={cn(treeRowClassName, 'pl-7')}>
      <Inbox {...ICON_PROPS} />
      <span className="min-w-0 flex-1 truncate">{foldersCopy.unfiled}</span>
      {count === 0 ? null : <span className="text-xs text-fg-subtle tabular-nums">{count}</span>}
    </Link>
  );
}

/** The whole Folders entry of the sidebar. */
export function FolderNavSection() {
  const tree = useFolderTree();
  const open = useFolderTreeStore((state) => state.open);
  const [naming, setNaming] = useState<FolderNameTarget>(null);
  const entries = tree.data?.entries ?? [];
  return (
    <div className="flex flex-col gap-px">
      <FoldersHeading
        entries={entries}
        onNew={() => {
          setNaming({ kind: 'create', parent: null });
        }}
      />
      {open && tree.data !== undefined ? (
        <>
          <Tree entries={entries} />
          <UnfiledLink count={tree.data.unfiledCount} />
        </>
      ) : null}
      <FolderNameDialog
        target={naming}
        onClose={() => {
          setNaming(null);
        }}
      />
    </div>
  );
}
