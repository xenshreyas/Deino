import { useRouter } from '@tanstack/react-router';

import { useRecordingRequests } from '@/features/detection';

/**
 * Opens the record screen when the user answers a meeting prompt with
 * "Take notes". Rust has already started the recording (titled after the
 * calendar event or the app) and brought the window forward; an auto-start
 * records without taking focus, so it opens nothing.
 *
 * Renders nothing.
 */
export function DetectionRecordingStart() {
  const router = useRouter();
  useRecordingRequests(({ openNotes }) => {
    if (openNotes) void router.navigate({ to: '/' });
  });
  return null;
}
