/**
 * Moments (TRN-04): a link to a time of a meeting's recording,
 * `deino://meeting/<id>?t=<ms>`, the shape the `deino` CLI, MCP tools and
 * exports print and Deino opens (DEV-05).
 */
import { toast } from 'sonner';

import { clipsCopy } from './copy';
import type { MeetingId } from './types';

/** The scheme and host of a meeting link. */
const MEETING_LINK = 'deino://meeting/';

/** `deino://meeting/<id>?t=<ms>`, the time rounded down to whole milliseconds. */
export function momentLink(meetingId: MeetingId, ms: number): string {
  const at = Math.max(0, Math.floor(ms));
  return `${MEETING_LINK}${encodeURIComponent(meetingId)}?t=${String(at)}`;
}

/** Copies the moment's link to the clipboard and says so. */
export async function copyMomentLink(meetingId: MeetingId, ms: number): Promise<void> {
  try {
    await navigator.clipboard.writeText(momentLink(meetingId, ms));
    toast.success(clipsCopy.linkCopied);
  } catch {
    toast.error(clipsCopy.linkFailed);
  }
}
