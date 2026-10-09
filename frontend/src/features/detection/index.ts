/**
 * Meeting detection (CAP-06): the prompt that appears when a call starts
 * (its own window, `detection-prompt.html`), the hook the shell uses to start
 * a recording when the user takes notes, and what Settings needs.
 */
export { detectionApi } from './api';
export { DetectionPrompt } from './components/detection-prompt';
export { MeetingPrompt, type MeetingPromptProps } from './components/meeting-prompt';
export { detectionCopy } from './copy';
export {
  currentDetectionQuery,
  detectionKeys,
  detectionOverviewQuery,
  useDetectionOverview,
  usePromptDetection,
  useRecordingRequests,
} from './queries';
export {
  type AppIdentity,
  asBundleId,
  type BundleId,
  type Detection,
  type DetectionId,
  type DetectionOverview,
  type IgnoredApp,
  type MeetingKind,
  type RecordingRequested,
} from './types';
