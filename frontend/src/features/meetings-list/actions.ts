/**
 * The page's actions on many meetings: trash (with Undo), restore, delete
 * forever, tag, file and merge. Plain functions over the API rather than
 * mutation hooks, so the keyboard shortcuts (a window listener) can run them
 * too; `DbChanged` refreshes every query they touch, and each reports its
 * outcome as a toast.
 */
import { toast } from 'sonner';

import type { FolderId, TagId } from '@/lib/ipc/generated/meetings';
import { IpcError } from '@/lib/ipc/result';

import { meetingsListApi } from './api';
import { copy } from './copy';
import { usePageStore } from './page-store';
import { selectAll } from './selection';
import type { MeetingId, MergeOutcome } from './types';

/** Shows why an action failed: Rust's messages are user-safe. */
function failed(reason: unknown): undefined {
  const error = IpcError.from(reason);
  toast.error(error.message === '' ? copy.bulk.failed : error.message);
  return undefined;
}

/** Takes `ids` back out of the trash. */
export async function restoreMeetings(ids: MeetingId[]): Promise<void> {
  const outcome = await meetingsListApi.restore(ids).catch(failed);
  if (outcome !== undefined) toast.success(copy.trash.restored(outcome.changed.length));
}

/** Moves `ids` to the trash, offering Undo. */
export async function trashMeetings(ids: MeetingId[]): Promise<void> {
  const outcome = await meetingsListApi.trash(ids).catch(failed);
  if (outcome === undefined) return;
  usePageStore.getState().clearSelection();
  toast.success(copy.trash.moved(outcome.changed.length), {
    action: { label: copy.trash.undo, onClick: () => void restoreMeetings(outcome.changed) },
  });
}

/** Moves the selected meetings to the trash (the bulk bar and ⌘⌫). */
export function trashSelection(): void {
  const ids = [...usePageStore.getState().selection.ids];
  if (ids.length > 0) void trashMeetings(ids);
}

/** Selects every meeting the filter on screen matches, loaded or not (⌘A). */
export async function selectAllMatching(): Promise<void> {
  const { filter } = usePageStore.getState();
  const ids = await meetingsListApi.matchingIds(filter).catch(failed);
  // The user may have changed the filter while the IDs loaded.
  if (ids !== undefined && usePageStore.getState().filter === filter) {
    usePageStore.getState().setSelection(selectAll(ids));
  }
}

/** Deletes trashed `ids` for good. */
export async function deleteForever(ids: MeetingId[]): Promise<void> {
  const outcome = await meetingsListApi.deleteForever(ids).catch(failed);
  if (outcome !== undefined) toast.success(copy.trash.deleted(outcome.changed.length));
}

/** Puts `tag` on `ids`. */
export async function tagMeetings(ids: MeetingId[], tag: TagId): Promise<void> {
  const outcome = await meetingsListApi.tagMany(ids, tag).catch(failed);
  if (outcome !== undefined) toast.success(copy.bulk.tagged(ids.length));
}

/** Files `ids` into `folder`. */
export async function fileMeetings(ids: MeetingId[], folder: FolderId): Promise<void> {
  const outcome = await meetingsListApi.fileMany(ids, folder).catch(failed);
  if (outcome !== undefined) toast.success(copy.bulk.filed(ids.length));
}

/** Merges `ids` into the earliest; resolves to the outcome, or `null` on failure. */
export async function mergeMeetings(ids: MeetingId[]): Promise<MergeOutcome | null> {
  const outcome = await meetingsListApi.merge(ids).catch(failed);
  if (outcome === undefined) return null;
  usePageStore.getState().clearSelection();
  toast.success(copy.merge.done);
  return outcome;
}
