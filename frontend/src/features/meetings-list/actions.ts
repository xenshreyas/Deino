/**
 * The page's actions on many meetings: trash (with Undo), restore, delete
 * forever, tag, file, export, merge and run an automation (AUT-01). Plain functions over the API rather than
 * mutation hooks, so the keyboard shortcuts (a window listener) can run them
 * too; `DbChanged` refreshes every query they touch. A toast reports only
 * what the list cannot show (UX-32): a trash with Undo, an export, a run.
 */
import { toast } from 'sonner';

import { type AutomationId, automationsApi } from '@/features/automations';
import { type ExportFormat, exportApi } from '@/features/export';
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

/** Takes `ids` back out of the trash; the rows come back on their own. */
export async function restoreMeetings(ids: MeetingId[]): Promise<void> {
  await meetingsListApi.restore(ids).catch(failed);
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

/** Deletes trashed `ids` for good; the confirm dialog already said so. */
export async function deleteForever(ids: MeetingId[]): Promise<void> {
  await meetingsListApi.deleteForever(ids).catch(failed);
}

/** Puts `tag` on `ids`; the rows show it. */
export async function tagMeetings(ids: MeetingId[], tag: TagId): Promise<void> {
  await meetingsListApi.tagMany(ids, tag).catch(failed);
}

/** Files `ids` into `folder`; the sidebar's count shows it. */
export async function fileMeetings(ids: MeetingId[], folder: FolderId): Promise<void> {
  await meetingsListApi.fileMany(ids, folder).catch(failed);
}

/** Runs `automation` on `ids` now, whatever its conditions (a run each, in the pipeline). */
export async function runAutomation(
  ids: MeetingId[],
  automation: { id: AutomationId; name: string },
): Promise<void> {
  const outcome = await automationsApi.run(automation.id, ids).catch(failed);
  if (outcome !== undefined) toast.success(copy.bulk.running(automation.name, ids.length));
}

/** Merges `ids` into the earliest; resolves to the outcome, or `null` on failure. */
export async function mergeMeetings(ids: MeetingId[]): Promise<MergeOutcome | null> {
  const outcome = await meetingsListApi.merge(ids).catch(failed);
  if (outcome === undefined) return null;
  usePageStore.getState().clearSelection();
  return outcome;
}

/** Writes `ids` as `format` into a folder the user picks (one file each). */
export async function exportMeetings(ids: MeetingId[], format: ExportFormat): Promise<void> {
  const saved = await exportApi.saveMany(ids, format).catch(failed);
  if (saved) toast.success(copy.bulk.exported(saved.count, saved.folder));
}
