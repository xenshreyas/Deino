/**
 * Typed calls to the `meetings` plugin. Each resolves to the data or throws
 * an `IpcError`; components use the hooks in `queries.ts` instead.
 */
import { commands } from '@/lib/ipc/generated/meetings';
import { unwrap } from '@/lib/ipc/result';
import { storageService } from '@/services/storageService';

import type { Meeting, MeetingId } from './types';

export const meetingsApi = {
  /** Every meeting, newest first. */
  list: (): Promise<Meeting[]> => unwrap(commands.list()),
  /** One meeting; rejects with `not_found` when it does not exist. */
  get: (id: MeetingId): Promise<Meeting> => unwrap(commands.get(id)),
  /** Renames a meeting; rejects with `invalid` for a blank or overlong title. */
  rename: (id: MeetingId, title: string): Promise<Meeting> => unwrap(commands.rename(id, title)),
  /**
   * Deletes a meeting with its transcript, summary and recording folder. Still
   * the legacy command until ORG-07 ports it here (with a trash).
   */
  delete: (id: MeetingId): Promise<void> => storageService.deleteMeeting(id),
};
