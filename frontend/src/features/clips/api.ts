/** Typed calls to the `clips` plugin (TRN-04). */
import { commands } from '@/lib/ipc/generated/clips';
import { unwrap } from '@/lib/ipc/result';

import type { Clip, ClipId, MeetingId, NewClip } from './types';

export const clipsApi = {
  /** Cuts a named range of the meeting's recording into a clip. */
  create: (clip: NewClip): Promise<Clip> => unwrap(commands.create(clip)),
  /** The meeting's clips, in time order. */
  list: (meetingId: MeetingId): Promise<Clip[]> => unwrap(commands.list(meetingId)),
  /** Every clip of every meeting not in the trash, newest first. */
  listAll: (): Promise<Clip[]> => unwrap(commands.listAll()),
  /** Renames the clip and its file. */
  rename: (id: ClipId, name: string): Promise<Clip> => unwrap(commands.rename(id, name)),
  /** Deletes the clip and its file. */
  delete: (id: ClipId): Promise<null> => unwrap(commands.delete(id)),
  /** Shows the clip's file in Finder. */
  reveal: (id: ClipId): Promise<null> => unwrap(commands.reveal(id)),
};
