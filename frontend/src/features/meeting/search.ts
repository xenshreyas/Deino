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
export const meetingTabSchema = z.enum(['notes', 'transcript']);

/** One of the meeting page's tabs. */
export type MeetingTabId = z.infer<typeof meetingTabSchema>;

/**
 * `?tab=` as links write it: the tabs, plus the Insights and Clips tabs
 * that were folded away (UX-03). Insight answers now end the Notes tab and
 * clips are in Details, so old links open Notes.
 */
const tabParamSchema = z
  .enum([...meetingTabSchema.options, 'insights', 'clips'])
  .transform((tab): MeetingTabId => (tab === 'insights' || tab === 'clips' ? 'notes' : tab));

/** A moment of the recording, in milliseconds from its start (TRN-01 deep links). */
export const momentSchema = z.coerce.number().int().nonnegative();

/**
 * `/meetings/$meetingId?source=recording&tab=transcript&t=221900`: `t` is a
 * moment in the recording, in milliseconds (an action item's evidence, an
 * ORG-08 search result); `segment=<id>` is the transcript segment a citation
 * points at (NOT-03 "Show in transcript"). With either and no `tab`, the page
 * opens on the Transcript tab, which scrolls to and marks the line (TRN-01);
 * the player (TRN-03) will seek to `t`.
 */
export const meetingSearchSchema = z.object({
  source: meetingSourceSchema.optional().catch(undefined),
  tab: tabParamSchema.optional().catch(undefined),
  t: momentSchema.optional().catch(undefined),
  segment: z.string().uuid().optional().catch(undefined),
});

/** The pre-Vite `/meeting-details?id=<id>&source=<source>` URL, kept as a redirect. */
export const legacyMeetingSearchSchema = z.object({
  source: meetingSourceSchema.optional().catch(undefined),
  id: z.string().min(1).optional().catch(undefined),
});
