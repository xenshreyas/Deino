/**
 * URL search parameters of the meeting routes, validated with zod.
 *
 * Unknown or malformed values fall back to "absent" instead of failing the
 * route, because the URLs come from the app itself and from old bookmarks.
 */
import { z } from 'zod';

/** Where a meeting was opened from. Only a fresh recording changes behaviour. */
export const meetingSourceSchema = z.literal('recording');

/** Origin of a navigation to a meeting. */
export type MeetingSource = z.infer<typeof meetingSourceSchema>;

/** `/meetings/$meetingId?source=recording` */
export const meetingSearchSchema = z.object({
  source: meetingSourceSchema.optional().catch(undefined),
});

/** The pre-Vite `/meeting-details?id=<id>&source=<source>` URL, kept as a redirect. */
export const legacyMeetingSearchSchema = meetingSearchSchema.extend({
  id: z.string().min(1).optional().catch(undefined),
});
