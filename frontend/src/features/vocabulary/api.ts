/**
 * Typed calls to the `vocabulary` plugin. Each resolves to the data or
 * throws an `IpcError`; components use the hooks in `queries.ts` instead.
 */
import { commands } from '@/lib/ipc/generated/vocabulary';
import { unwrap } from '@/lib/ipc/result';

import type {
  CorrectedSegment,
  MeetingId,
  SegmentId,
  TermInput,
  UndoReport,
  VocabTerm,
  VocabTermId,
} from './types';

export const vocabularyApi = {
  /** Every entry: replacements, then terms, each by spelling. */
  list: (): Promise<VocabTerm[]> => unwrap(commands.list()),
  /** Adds a term or replacement. */
  create: (input: TermInput): Promise<VocabTerm> => unwrap(commands.create(input)),
  /** Replaces an entry's kind, spelling and sounds-like spellings. */
  update: (id: VocabTermId, input: TermInput): Promise<VocabTerm> =>
    unwrap(commands.update(id, input)),
  /** Deletes an entry. */
  delete: (id: VocabTermId): Promise<null> => unwrap(commands.delete(id)),
  /** The meeting's corrections, in transcript order. */
  corrections: (meetingId: MeetingId): Promise<CorrectedSegment[]> =>
    unwrap(commands.corrections(meetingId)),
  /** Undoes one segment's correction, or all of the meeting's. */
  undo: (meetingId: MeetingId, segmentId: SegmentId | null): Promise<UndoReport> =>
    unwrap(commands.undo(meetingId, segmentId)),
};
