/**
 * The meeting page: header and its ⋯ menu, tabs (Notes, Transcript), right
 * panel (Chat, Details) and player, each a slot in `slots.ts`.
 * Routes import its URL contract from `./search`.
 */
export { MeetingScreen, type MeetingScreenProps } from './components/meeting-screen';
export { MEETING_PANELS, MEETING_PLAYER, MEETING_TABS } from './slots';
export type { MeetingMoment, MeetingPanelId, MeetingSlotProps, MeetingTabId } from './types';
