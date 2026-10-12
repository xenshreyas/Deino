/**
 * Screen capture (CAP-09): snapshots and video of the meeting window while
 * Deino records. Settings → Screen capture, the meeting page's Images
 * strip, and in the record panel the window or screen picker and the
 * consent reminder shown the first three times a capture runs.
 */
export { screenApi, snapshotSrc } from './api';
export { captureMenuItems, sameSource } from './capture-menu';
export { ImagesStripView, MeetingImages, snapshotTime } from './components/images-strip';
export {
  capturedMeeting,
  type ScreenReminderState,
  useScreenReminder,
} from './components/screen-reminder';
export { ScreenCaptureSettings } from './components/screen-settings';
export { screenCopy } from './copy';
export {
  screenKeys,
  screenStatusQuery,
  snapshotsQuery,
  useOpenScreenRecordingSettings,
  useScreenStatus,
  useTurnOffForMeeting,
} from './queries';
export type {
  CaptureSource,
  CaptureState,
  CaptureStatus,
  ScreenSettings,
  ScreenStatus,
  Snapshot,
  TargetChoice,
} from './types';
