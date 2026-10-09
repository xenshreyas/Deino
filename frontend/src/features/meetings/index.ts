/**
 * The meetings feature: list, open, rename and delete meetings through the
 * `meetings` plugin, and the Meetings page. Other code imports from here only.
 */
export { meetingsApi } from './api';
export { MeetingsScreen, type MeetingsScreenProps } from './components/meetings-screen';
export {
  meetingKeys,
  meetingListQuery,
  meetingQuery,
  useDeleteMeeting,
  useMeeting,
  useMeetings,
  useRenameMeeting,
} from './queries';
export { asMeetingId, type Meeting, type MeetingId, type RenameMeetingInput } from './types';
