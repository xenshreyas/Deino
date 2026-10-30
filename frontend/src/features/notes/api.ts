/**
 * Typed calls to the `enhance` plugin (NOT-02, NOT-03): the enhanced notes'
 * only IPC. Each resolves to the data or throws an `IpcError`.
 */
import type { MeetingId } from '@/features/meetings';
import type { NoteDoc } from '@/features/notepad';
import { commands } from '@/lib/ipc/generated/enhance';
import { unwrap } from '@/lib/ipc/result';

import type {
  CitedSegment,
  EnhancedNotesView,
  NotesDraft,
  SavedEnhancedNotes,
  SegmentId,
  VersionInfo,
} from './types';

export const enhanceApi = {
  /** A version of the notes (the newest for `null`); `null` before the first. */
  get: (meetingId: MeetingId, version: number | null): Promise<EnhancedNotesView | null> =>
    unwrap(commands.get(meetingId, version)),
  /** Every version, newest first. */
  versions: (meetingId: MeetingId): Promise<VersionInfo[]> => unwrap(commands.versions(meetingId)),
  /** Writes a new version; `conflict` when it would replace edits without consent. */
  enhance: (meetingId: MeetingId, replaceEdits: boolean): Promise<EnhancedNotesView> =>
    unwrap(commands.enhance(meetingId, replaceEdits)),
  /** The notes as far as they are written, while they are; `null` otherwise. */
  draft: (meetingId: MeetingId): Promise<NotesDraft | null> => unwrap(commands.draft(meetingId)),
  /** Stops writing the notes; nothing is saved. `false` when none were being written. */
  stop: (meetingId: MeetingId): Promise<boolean> => unwrap(commands.stop(meetingId)),
  /** Saves an edit of the newest version. */
  save: (meetingId: MeetingId, version: number, doc: NoteDoc): Promise<SavedEnhancedNotes> =>
    unwrap(commands.save(meetingId, version, doc)),
  /** Makes a copy of an older version the newest. */
  restore: (meetingId: MeetingId, version: number): Promise<EnhancedNotesView> =>
    unwrap(commands.restore(meetingId, version)),
  /** The transcript excerpts behind citations. */
  cited: (meetingId: MeetingId, segmentIds: SegmentId[]): Promise<CitedSegment[]> =>
    unwrap(commands.cited(meetingId, segmentIds)),
};
