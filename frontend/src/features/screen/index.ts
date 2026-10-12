/**
 * Screen capture (CAP-09): snapshots and video of the meeting window while
 * Deino records a detected meeting. The UI is one Settings page for now;
 * the record panel's picker and the meeting's Images strip build on the
 * same queries.
 */
export { screenApi } from './api';
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
export type { CaptureState, CaptureStatus, ScreenSettings, ScreenStatus, Snapshot } from './types';
