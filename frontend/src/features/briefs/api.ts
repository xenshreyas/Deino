/**
 * Typed calls to the `briefs` plugin. Each resolves to the data or throws an
 * `IpcError`; components use the hooks in `queries.ts` instead.
 */
import type { CalendarEventId } from '@/lib/ipc/generated/calendar';
import { commands } from '@/lib/ipc/generated/briefs';
import type { MeetingId } from '@/lib/ipc/generated/meetings';
import { unwrap } from '@/lib/ipc/result';

import type { Brief, BriefFeedback } from './types';

export const briefsApi = {
  /** The brief of an upcoming event, or `null` when there is none. */
  forEvent: (event: CalendarEventId): Promise<Brief | null> => unwrap(commands.forEvent(event)),
  /** The brief of the event a meeting note is linked to, or `null`. */
  forMeeting: (meeting: MeetingId): Promise<Brief | null> => unwrap(commands.forMeeting(meeting)),
  /** Records that the user saw the brief. */
  markRead: (event: CalendarEventId): Promise<null> => unwrap(commands.markRead(event)),
  /** Stores the user's verdict. */
  giveFeedback: (event: CalendarEventId, feedback: BriefFeedback): Promise<null> =>
    unwrap(commands.giveFeedback(event, feedback)),
};
