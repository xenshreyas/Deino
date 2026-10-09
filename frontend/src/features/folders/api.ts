/**
 * Typed calls to the `folders` plugin. Each resolves to the data or throws
 * an `IpcError`; components use the hooks in `queries.ts` instead.
 */
import { commands } from '@/lib/ipc/generated/folders';
import { unwrap } from '@/lib/ipc/result';

import type {
  FiledMeeting,
  Folder,
  FolderId,
  FolderRule,
  FolderRuleEntry,
  FolderRuleId,
  FolderTree,
  Meeting,
  MeetingId,
  NewFolder,
  UnfiledMeeting,
} from './types';

export const foldersApi = {
  /** Every folder in display order, with counts and the Unfiled count. */
  tree: (): Promise<FolderTree> => unwrap(commands.tree()),
  /** One folder; rejects with `not_found`. */
  get: (id: FolderId): Promise<Folder> => unwrap(commands.get(id)),
  /** Creates a folder; rejects with `invalid` or `conflict` (a sibling's name). */
  create: (folder: NewFolder): Promise<Folder> => unwrap(commands.create(folder)),
  /** Renames a folder. */
  rename: (id: FolderId, name: string): Promise<Folder> => unwrap(commands.rename(id, name)),
  /** Sets or clears a folder's icon. */
  setIcon: (id: FolderId, icon: string | null): Promise<Folder> =>
    unwrap(commands.setIcon(id, icon)),
  /** Sets what a folder is for. */
  setDescription: (id: FolderId, description: string): Promise<Folder> =>
    unwrap(commands.setDescription(id, description)),
  /** Moves a folder (and everything in it) under a parent, or to the top. */
  moveTo: (id: FolderId, parentId: FolderId | null): Promise<Folder> =>
    unwrap(commands.moveTo(id, parentId)),
  /** Deletes a folder and its subfolders; every meeting stays. */
  delete: (id: FolderId): Promise<null> => unwrap(commands.delete(id)),
  /** The meetings in a folder, newest first. */
  meetings: (id: FolderId): Promise<Meeting[]> => unwrap(commands.meetings(id)),
  /** The meetings in no folder, newest first. */
  unfiled: (): Promise<Meeting[]> => unwrap(commands.unfiled()),
  /** The folders a meeting sits in. */
  foldersOf: (meetingId: MeetingId): Promise<Folder[]> => unwrap(commands.foldersOf(meetingId)),
  /** Files a meeting into a folder; may offer its recurring series. */
  addMeeting: (folderId: FolderId, meetingId: MeetingId): Promise<FiledMeeting> =>
    unwrap(commands.addMeeting(folderId, meetingId)),
  /** Takes a meeting out of a folder. */
  removeMeeting: (folderId: FolderId, meetingId: MeetingId): Promise<UnfiledMeeting> =>
    unwrap(commands.removeMeeting(folderId, meetingId)),
  /** A folder's auto-add rules. */
  rules: (folderId: FolderId): Promise<FolderRuleEntry[]> => unwrap(commands.rules(folderId)),
  /** Adds an auto-add rule. */
  addRule: (folderId: FolderId, rule: FolderRule): Promise<FolderRuleEntry> =>
    unwrap(commands.addRule(folderId, rule)),
  /** Deletes an auto-add rule. */
  removeRule: (id: FolderRuleId): Promise<null> => unwrap(commands.removeRule(id)),
};
