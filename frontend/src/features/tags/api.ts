/**
 * Typed calls to the `tags` plugin. Each resolves to the data or throws an
 * `IpcError`; components use the hooks in `queries.ts` instead.
 */
import type { MeetingId } from '@/lib/ipc/generated/meetings';
import { commands } from '@/lib/ipc/generated/tags';
import { unwrap } from '@/lib/ipc/result';

import type { AppliedTag, MeetingTagLink, NewTag, Tag, TagId, TagUpdate } from './types';

export const tagsApi = {
  /** Every tag, by name. */
  list: (): Promise<Tag[]> => unwrap(commands.list()),
  /** Creates a tag; rejects with `invalid` or `conflict` (a taken name). */
  create: (tag: NewTag): Promise<Tag> => unwrap(commands.create(tag)),
  /** Replaces a tag's name, colour, description and auto-apply switch. */
  update: (id: TagId, update: TagUpdate): Promise<Tag> => unwrap(commands.update(id, update)),
  /** Deletes a tag; it leaves every meeting. */
  delete: (id: TagId): Promise<null> => unwrap(commands.delete(id)),
  /** The tags on a meeting, with who applied them. */
  tagsOf: (meetingId: MeetingId): Promise<AppliedTag[]> => unwrap(commands.tagsOf(meetingId)),
  /** Every tag on every meeting, for the Meetings page. */
  links: (): Promise<MeetingTagLink[]> => unwrap(commands.links()),
  /** Puts a tag on a meeting by hand. */
  apply: (meetingId: MeetingId, tagId: TagId): Promise<null> =>
    unwrap(commands.apply(meetingId, tagId)),
  /** Takes a tag off a meeting. */
  remove: (meetingId: MeetingId, tagId: TagId): Promise<null> =>
    unwrap(commands.remove(meetingId, tagId)),
};
