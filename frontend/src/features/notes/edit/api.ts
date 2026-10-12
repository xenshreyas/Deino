/**
 * Typed calls to the `notes-edit` plugin (NOT-06): propose an edit of the
 * newest enhanced notes from the user's words, and accept one. Undo is the
 * `enhance` plugin's restore of the previous version.
 */
import type { MeetingId } from '@/features/meetings';
import type { NoteDoc } from '@/features/notepad';
import { commands } from '@/lib/ipc/generated/notes-edit';
import { unwrap } from '@/lib/ipc/result';

import type { AcceptedEdit, EditProposal, EditRequest } from './types';

export const noteEditApi = {
  /** The edit the request asks for, with its diff; nothing is stored. */
  propose: (request: EditRequest): Promise<EditProposal> => unwrap(commands.propose(request)),
  /** Stores an accepted proposal as the newest version; `conflict` once stale. */
  accept: (meetingId: MeetingId, baseVersion: number, doc: NoteDoc): Promise<AcceptedEdit> =>
    unwrap(commands.accept(meetingId, baseVersion, doc)),
};
