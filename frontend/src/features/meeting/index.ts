/**
 * The meeting page: header, tabs (Notes, Transcript, Insights, Clips), right
 * panel (Chat, Participants, Details) and player, each a slot in `slots.ts`.
 * Routes import its URL contract from `./search`.
 */
export { MeetingScreen, type MeetingScreenProps } from './components/meeting-screen';
export { MEETING_PANELS, MEETING_PLAYER, MEETING_TABS } from './slots';
export type { MeetingMoment, MeetingPanelId, MeetingSlotProps, MeetingTabId } from './types';
