/**
 * Typed calls to the `followups` plugin. Each resolves to the data or
 * throws an `IpcError`; components use the hooks in `queries.ts` instead.
 */
import { commands } from '@/lib/ipc/generated/followups';
import type { MeetingId } from '@/lib/ipc/generated/meetings';
import { unwrap } from '@/lib/ipc/result';

import type { FollowUpDraft, FollowUpEdit, FollowUpSummary } from './types';

export const followUpsApi = {
  /** Home's suggestions: ready drafts of the last three days. */
  suggested: (): Promise<FollowUpSummary[]> => unwrap(commands.suggested()),
  /** The meeting's ready draft, or `null`. */
  forMeeting: (meeting: MeetingId): Promise<FollowUpDraft | null> =>
    unwrap(commands.forMeeting(meeting)),
  /** Stores the user's edit. */
  save: (meeting: MeetingId, edit: FollowUpEdit): Promise<FollowUpDraft> =>
    unwrap(commands.save(meeting, edit)),
  /** Dismisses the follow-up for good. */
  dismiss: (meeting: MeetingId): Promise<null> => unwrap(commands.dismiss(meeting)),
  /** Saves the edit and opens it in Mail. */
  openInMail: (meeting: MeetingId, edit: FollowUpEdit): Promise<null> =>
    unwrap(commands.openInMail(meeting, edit)),
  /** Saves the edit and copies its body. */
  copy: (meeting: MeetingId, edit: FollowUpEdit): Promise<null> =>
    unwrap(commands.copy(meeting, edit)),
};
