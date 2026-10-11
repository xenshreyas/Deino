/**
 * Editing and deleting one line (TRN-02), optimistic: the cached transcript
 * changes at once and Rust confirms (its `DbChanged` refetches the real
 * rows); a failure puts the old rows back and says so. A delete offers Undo
 * in its toast, which restores the line.
 */
import type { QueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';

import { transcriptApi } from '../api';
import { transcriptCopy } from '../copy';
import { transcriptKeys } from '../queries';
import type { MeetingId, Segment, SegmentId, Transcript } from '../types';

/** What an edit works on. */
export interface LineTarget {
  client: QueryClient;
  meetingId: MeetingId;
  segmentId: SegmentId;
}

/** Applies `change` to the cached transcript; returns the rows before it. */
function patchCache(
  { client, meetingId }: LineTarget,
  change: (segments: Segment[]) => Segment[],
): Transcript | undefined {
  const key = transcriptKeys.meeting(meetingId);
  const before = client.getQueryData<Transcript>(key);
  if (before !== undefined) client.setQueryData(key, { segments: change(before.segments) });
  return before;
}

/** Puts `before` back and refetches, after a failed write. */
function rollback({ client, meetingId }: LineTarget, before: Transcript | undefined): void {
  const key = transcriptKeys.meeting(meetingId);
  if (before !== undefined) client.setQueryData(key, before);
  void client.invalidateQueries({ queryKey: key });
}

/** Sets the line's text; resolves `false` (after saying why) when it was not saved. */
export async function saveLineText(target: LineTarget, text: string): Promise<boolean> {
  const trimmed = text.trim();
  if (trimmed === '') {
    toast.error(transcriptCopy.emptyLine);
    return false;
  }
  const before = patchCache(target, (segments) =>
    segments.map((one) => (one.id === target.segmentId ? { ...one, text: trimmed } : one)),
  );
  try {
    await transcriptApi.edit(target.segmentId, trimmed);
    return true;
  } catch {
    rollback(target, before);
    toast.error(transcriptCopy.editFailed);
    return false;
  }
}

/** Restores a deleted line (the toast's Undo). */
async function restoreLine(target: LineTarget): Promise<void> {
  try {
    await transcriptApi.restore(target.segmentId);
  } catch {
    toast.error(transcriptCopy.restoreFailed);
  }
}

/** Deletes the line, with Undo in the toast. */
export async function deleteLine(target: LineTarget): Promise<void> {
  const before = patchCache(target, (segments) =>
    segments.filter((one) => one.id !== target.segmentId),
  );
  try {
    await transcriptApi.delete(target.segmentId);
    toast(transcriptCopy.deleted, {
      action: {
        label: transcriptCopy.undo,
        onClick: () => void restoreLine(target),
      },
    });
  } catch {
    rollback(target, before);
    toast.error(transcriptCopy.deleteFailed);
  }
}
