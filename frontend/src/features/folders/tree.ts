/**
 * Pure helpers over the folder tree as Rust sends it: entries in display
 * order (each folder followed by its subfolders, with a depth), so
 * collapsing, keyboard moves and drop rules are list walks, not recursion.
 */
import type { FolderId, FolderTreeEntry } from './types';

/** The entries left visible when the folders in `collapsed` are closed. */
export function visibleEntries(
  entries: readonly FolderTreeEntry[],
  collapsed: ReadonlySet<FolderId>,
): FolderTreeEntry[] {
  const visible: FolderTreeEntry[] = [];
  let hiddenBelow: number | null = null;
  for (const entry of entries) {
    if (hiddenBelow !== null && entry.depth > hiddenBelow) continue;
    hiddenBelow = collapsed.has(entry.folder.id) ? entry.depth : null;
    visible.push(entry);
  }
  return visible;
}

/** True when the entry at `index` has subfolders (the next entry is deeper). */
export function hasChildren(entries: readonly FolderTreeEntry[], index: number): boolean {
  const entry = entries[index];
  const next = entries[index + 1];
  return entry !== undefined && next !== undefined && next.depth > entry.depth;
}

/** The folders that have subfolders. */
export function parentIds(entries: readonly FolderTreeEntry[]): Set<FolderId> {
  return new Set(
    entries.flatMap((entry) => (entry.folder.parentId === null ? [] : [entry.folder.parentId])),
  );
}

/** `id` and every folder below it. */
export function subtreeIds(entries: readonly FolderTreeEntry[], id: FolderId): Set<FolderId> {
  const ids = new Set<FolderId>();
  const start = entries.findIndex((entry) => entry.folder.id === id);
  const root = entries[start];
  if (root === undefined) return ids;
  ids.add(id);
  for (const entry of entries.slice(start + 1)) {
    if (entry.depth <= root.depth) break;
    ids.add(entry.folder.id);
  }
  return ids;
}

/**
 * True when `folder` may be dropped into `target` (`null`: the top level):
 * not into itself or its own subfolders, and not where it already is.
 */
export function canMoveFolder(
  entries: readonly FolderTreeEntry[],
  folder: FolderId,
  target: FolderId | null,
): boolean {
  const moving = entries.find((entry) => entry.folder.id === folder);
  if (moving === undefined || moving.folder.parentId === target) return false;
  return target === null || !subtreeIds(entries, folder).has(target);
}

/** The folders `folder` may move into, in display order. */
export function moveTargets(
  entries: readonly FolderTreeEntry[],
  folder: FolderId,
): FolderTreeEntry[] {
  return entries.filter((entry) => canMoveFolder(entries, folder, entry.folder.id));
}

/** The names from the top level down to `id`, for "Work / Clients". */
export function folderPath(entries: readonly FolderTreeEntry[], id: FolderId): string[] {
  const index = entries.findIndex((entry) => entry.folder.id === id);
  const path: string[] = [];
  let depth = Number.POSITIVE_INFINITY;
  for (const entry of entries.slice(0, index + 1).reverse()) {
    if (entry.depth >= depth) continue;
    path.unshift(entry.folder.name);
    depth = entry.depth;
  }
  return path;
}

/** The folder above `id`, or `null` at the top level or for an unknown folder. */
export function parentOf(entries: readonly FolderTreeEntry[], id: FolderId): FolderId | null {
  return entries.find((entry) => entry.folder.id === id)?.folder.parentId ?? null;
}
