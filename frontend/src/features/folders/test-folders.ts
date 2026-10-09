/**
 * Folder values for this feature's tests. Not exported from `index.ts`, so
 * nothing outside the tests reaches it.
 */
import type { FolderId, FolderTreeEntry } from './types';

const AT = '2026-10-09T09:00:00.000Z';

/** A tree entry for the folder `key`, under the folder `parent` (`null`: the top). */
export function folderEntry(
  key: string,
  name: string,
  parent: string | null,
  depth: number,
): FolderTreeEntry {
  return {
    folder: {
      id: `folder-${key}` as FolderId,
      parentId: parent === null ? null : (`folder-${parent}` as FolderId),
      name,
      icon: null,
      description: '',
      createdAt: AT,
      updatedAt: AT,
    },
    depth,
    meetingCount: 0,
  };
}
