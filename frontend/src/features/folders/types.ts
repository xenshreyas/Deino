/**
 * The folders feature's types, generated from Rust by `bindings.rs`, plus
 * the brand check for folder IDs that arrive as route params.
 */
import type { FolderId } from '@/lib/ipc/generated/folders';

export type {
  FiledMeeting,
  Folder,
  FolderId,
  FolderRule,
  FolderRuleEntry,
  FolderRuleId,
  FolderTree,
  FolderTreeEntry,
  Meeting,
  MeetingId,
  NewFolder,
  SeriesId,
  TagId,
  UnfiledMeeting,
} from '@/lib/ipc/generated/folders';

/** A folder ID from a URL. Folder IDs are UUIDs minted by Rust. */
export function asFolderId(raw: string): FolderId {
  if (raw.trim() === '') throw new TypeError('A folder ID cannot be empty.');
  return raw as FolderId;
}
