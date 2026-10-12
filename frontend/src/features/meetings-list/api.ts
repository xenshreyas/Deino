/**
 * Typed calls to the `meetings` plugin's list commands (ORG-07). Each
 * resolves to the data or throws an `IpcError`; components use the hooks in
 * `queries.ts` instead.
 */
import type { FolderId, TagId } from '@/lib/ipc/generated/meetings';
import { commands } from '@/lib/ipc/generated/meetings';
import { unwrap } from '@/lib/ipc/result';

import type {
  MeetingCursor,
  MeetingFilter,
  MeetingId,
  MeetingViewId,
  MeetingViewInput,
} from './types';

export const meetingsListApi = {
  /** Up to `size` meetings `filter` matches, newest first, after `after`. */
  page: (filter: MeetingFilter, after: MeetingCursor | null, size: number) =>
    unwrap(commands.page(filter, after, size)),
  /** Every meeting `filter` matches (⌘A). */
  matchingIds: (filter: MeetingFilter) => unwrap(commands.matchingIds(filter)),
  /** The saved views, in tab order. */
  views: () => unwrap(commands.views()),
  /** Saves a new view after the last tab. */
  createView: (view: MeetingViewInput) => unwrap(commands.createView(view)),
  /** Renames a view or replaces its filter. */
  updateView: (id: MeetingViewId, view: MeetingViewInput) => unwrap(commands.updateView(id, view)),
  /** Deletes a view. */
  deleteView: (id: MeetingViewId) => unwrap(commands.deleteView(id)),
  /** Moves meetings to the trash. */
  trash: (ids: MeetingId[]) => unwrap(commands.trash(ids)),
  /** Takes meetings out of the trash. */
  restore: (ids: MeetingId[]) => unwrap(commands.restore(ids)),
  /** The trash, most recently trashed first. */
  trashed: () => unwrap(commands.trashed()),
  /** Deletes trashed meetings for good, with their recordings. */
  deleteForever: (ids: MeetingId[]) => unwrap(commands.deleteForever(ids)),
  /** Puts a tag on every meeting. */
  tagMany: (ids: MeetingId[], tag: TagId) => unwrap(commands.tagMany(ids, tag)),
  /** Files every meeting into a folder. */
  fileMany: (ids: MeetingId[], folder: FolderId) => unwrap(commands.fileMany(ids, folder)),
  /** Merges two or more meetings into the earliest. */
  merge: (ids: MeetingId[]) => unwrap(commands.merge(ids)),
};
