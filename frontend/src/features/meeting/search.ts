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

/** The meeting page's tabs, in order (the slots registry renders them). */
export const meetingTabSchema = z.enum(['notes', 'transcript', 'insights', 'clips']);

/** One of the meeting page's tabs. */
export type MeetingTabId = z.infer<typeof meetingTabSchema>;

/**
 * `/meetings/$meetingId?source=recording&tab=transcript&t=221900`: `t` is a
 * moment in the recording, in milliseconds (an action item's evidence); the
 * transcript and player cards (TRN-01, TRN-03) scroll and seek to it.
 * `segment=<id>` is the transcript segment a citation points at (NOT-03),
 * for the Transcript tab to bring into view.
 */
export const meetingSearchSchema = z.object({
  source: meetingSourceSchema.optional().catch(undefined),
  tab: meetingTabSchema.optional().catch(undefined),
  t: z.number().int().nonnegative().optional().catch(undefined),
  segment: z.string().uuid().optional().catch(undefined),
});

/** The pre-Vite `/meeting-details?id=<id>&source=<source>` URL, kept as a redirect. */
export const legacyMeetingSearchSchema = z.object({
  source: meetingSourceSchema.optional().catch(undefined),
  id: z.string().min(1).optional().catch(undefined),
});
