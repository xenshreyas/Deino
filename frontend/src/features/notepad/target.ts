/**
 * Where a notepad writes. A meeting page writes into its meeting; the
 * record screen writes into a draft it creates on the first edit
 * (`live-notes.ts`). The editor only sees this interface.
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
  /** The meeting to write into; a draft is created on first use. */
  resolve: () => Promise<OpenTarget>;
  /** The folder images resolve against, once known. */
  folder: () => string | null;
}

/** The target of a meeting that already exists. */
export function meetingTarget(notes: MeetingNotes): NoteTarget {
  const open: OpenTarget = { meetingId: notes.meetingId, folder: notes.folder };
  return { resolve: () => Promise.resolve(open), folder: () => open.folder };
}
