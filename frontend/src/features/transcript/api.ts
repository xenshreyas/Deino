/** Typed calls to the `transcript` plugin, the feature's only commands. */
import { commands } from '@/lib/ipc/generated/transcript';
import { unwrap } from '@/lib/ipc/result';

import type {
  MeetingId,
  ReplaceOutcome,
  ReplacePreview,
  ReplaceRequest,
  Segment,
  SegmentId,
  Transcript,
} from './types';

export const transcriptApi = {
  /** The meeting's saved segments in spoken order, each with its speaker ID. */
  get: (meetingId: MeetingId): Promise<Transcript> => unwrap(commands.get(meetingId)),
  /** Sets a line's text (trimmed, not blank); the first edit keeps the original. */
  edit: (segmentId: SegmentId, text: string): Promise<Segment> =>
    unwrap(commands.edit(segmentId, text)),
  /** Deletes a line from the transcript, the notes' input, search and exports. */
  delete: (segmentId: SegmentId): Promise<null> => unwrap(commands.delete(segmentId)),
  /** Brings a deleted line back. */
  restore: (segmentId: SegmentId): Promise<null> => unwrap(commands.restore(segmentId)),
  /** What a find and replace would change, with counts per meeting. */
  preview: (request: ReplaceRequest): Promise<ReplacePreview> => unwrap(commands.preview(request)),
  /** Replaces every match, in one transaction. */
  replace: (request: ReplaceRequest, replacement: string): Promise<ReplaceOutcome> =>
    unwrap(commands.replace(request, replacement)),
};
