/**
 * The meetings feature: list, open and rename meetings through the
 * `meetings` plugin. Other code imports from here only.
 */
export { meetingsApi } from './api';
export {
  meetingKeys,
  meetingListQuery,
  meetingQuery,
  useMeeting,
  useMeetings,
  useRenameMeeting,
} from './queries';
export { asMeetingId, type Meeting, type MeetingId, type RenameMeetingInput } from './types';
