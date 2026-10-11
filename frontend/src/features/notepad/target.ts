/**
 * Where a notepad writes. Today every notepad writes into an existing
 * meeting (the record screen's is the meeting being recorded). The editor
 * only sees this interface.
 */
import type { MeetingId } from '@/features/meetings';

import type { MeetingNotes } from './types';

/** An existing meeting and its folder, which image sources are relative to. */
export interface OpenTarget {
  meetingId: MeetingId;
  folder: string;
}

/** A notepad's destination. */
export interface NoteTarget {
  /** The meeting to write into. */
  resolve: () => Promise<OpenTarget>;
  /** The folder images resolve against, once known. */
  folder: () => string | null;
}

/** The target of a meeting that already exists. */
export function meetingTarget(notes: MeetingNotes): NoteTarget {
  const open: OpenTarget = { meetingId: notes.meetingId, folder: notes.folder };
  return { resolve: () => Promise.resolve(open), folder: () => open.folder };
}
