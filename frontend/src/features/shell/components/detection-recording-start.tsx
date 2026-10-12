import { useRouter } from '@tanstack/react-router';

import { type Detection, useRecordingRequests } from '@/features/detection';
import { askPermissionOnce } from '@/features/onboarding';

/** The meeting apps whose speaker names Deino reads through Accessibility (CAP-05). */
const NAMES_SPEAKERS: ReadonlySet<Detection['platform']> = new Set(['zoom', 'teams', 'googleMeet']);

/**
 * Opens the record screen when the user answers a meeting prompt with
 * "Take notes". Rust has already started the recording (titled after the
 * calendar event or the app) and brought the window forward; an auto-start
 * records without taking focus, so it opens nothing.
 *
 * The first Zoom, Teams or Meet call is also where Accessibility is asked
 * for (UX-22), since that is when speaker names start to matter.
 *
 * Renders nothing.
 */
export function DetectionRecordingStart() {
  const router = useRouter();
  useRecordingRequests(({ openNotes, detection }) => {
    if (openNotes) void router.navigate({ to: '/' });
    if (NAMES_SPEAKERS.has(detection.platform)) void askPermissionOnce('accessibility');
  });
  return null;
}
