/**
 * The meeting page's slot registry (PARITY_PLAN §4.6, FND-10).
 *
 * Feature cards fill the meeting page by editing this file only: replace a
 * placeholder's `Component`, or add a tab, panel or rail section. The page
 * renders the tabs in order, the right-panel sections as its tabs, the
 * context rail's sections top to bottom beside the column, and the player at
 * the bottom. UX-03 and UX-04 cut it to two tabs and two panel sections;
 * add to a slot before adding a new one.
 *
 * | Slot | Today | Filled by |
 * |---|---|---|
 * | Notes tab | action items, then Enhanced (NOT-02, NOT-03) or My notes (NOT-01) with the template menu; insight answers (AI-06) and screen images (CAP-09) after the notes when there are any | |
 * | Transcript tab | transcript view (`features/transcript`) | TRN-01 |
 * | Chat panel | "Ask about this meeting" (`features/ask`): questions and `/` recipes, cited, a thread per meeting | AI-02-lite, AI-04 (D13); AI-02 for cross-meeting |
 * | Context rail | on a wide window with the right panel closed: people with talk time, the action items, the follow-up draft (NOT-07) when there is one | |
 * | Details panel | the follow-up draft (NOT-07) when there is one, participants (CAP-04), comments (TRN-06) and clips (TRN-04) when there are any, calendar event and related meetings (ORG-03), corrections (TRN-05) collapsed | |
 * | Player | the docked player (`features/player`), seeking to `?t=` | |
 */
import type { ComponentType } from 'react';

import { CompactActionItems } from '@/features/action-items';
import { AskPanel } from '@/features/ask';
import { MeetingFollowUp } from '@/features/followups';

import { meetingCopy } from './copy';
import { DetailsPanelSlot } from './slots/details-panel';
import { NotesSlot } from './slots/notes-slot';
import { PlayerSlot } from './slots/player-slot';
import { RailPeople } from './slots/rail-people';
import { TranscriptSlot } from './slots/transcript-slot';
import type { MeetingPanelSlot, MeetingRailSlot, MeetingSlotProps, MeetingTabSlot } from './types';

/** The tabs, in order. */
export const MEETING_TABS: readonly MeetingTabSlot[] = [
  { id: 'notes', label: meetingCopy.notes, shortcut: '⌘1', Component: NotesSlot },
  { id: 'transcript', label: meetingCopy.transcript, shortcut: '⌘2', Component: TranscriptSlot },
];

/** The right panel's sections, in order. */
export const MEETING_PANELS: readonly MeetingPanelSlot[] = [
  { id: 'chat', label: meetingCopy.chat, Component: AskPanel },
  { id: 'details', label: meetingCopy.details, Component: DetailsPanelSlot },
];

/** The context rail's sections, top to bottom (each shows nothing when it has nothing). */
export const MEETING_RAIL: readonly MeetingRailSlot[] = [
  { id: 'people', Component: RailPeople },
  { id: 'action-items', Component: CompactActionItems },
  { id: 'follow-up', Component: MeetingFollowUp },
];

/** The bottom player. */
export const MEETING_PLAYER: ComponentType<MeetingSlotProps> = PlayerSlot;
