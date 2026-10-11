/**
 * The meeting page's slot registry (PARITY_PLAN §4.6, FND-10).
 *
 * Feature cards fill the meeting page by editing this file only: replace a
 * placeholder's `Component`, or add a tab or panel section. The page renders
 * the tabs in order, the right-panel sections as its tabs, and the player at
 * the bottom.
 *
 * | Slot | Today | Filled by |
 * |---|---|---|
 * | Notes tab | Enhanced (NOT-02, NOT-03) and My notes (NOT-01) | |
 * | Transcript tab | transcript view (`features/transcript`), with the inherited folder and re-transcribe tools | TRN-01 |
 * | Insights tab | each insight's answers, attributed (AI-06) | |
 * | Clips tab | empty state | TRN-04 |
 * | Chat panel | empty state | AI-02 |
 * | Participants panel | speakers, talk time, naming (CAP-04) | ORG-04 (attendees) |
 * | Details panel | created and edited dates, calendar event, related meetings (ORG-03) | |
 * | Player | resting bar | TRN-03 |
 */
import type { ComponentType } from 'react';

import { MeetingInsights } from '@/features/insights';
import { ParticipantsPanel } from '@/features/speakers';

import { meetingCopy } from './copy';
import { DetailsPanelSlot } from './slots/details-panel';
import { NotesSlot } from './slots/notes-slot';
import { ChatPanelSlot, ClipsSlot } from './slots/placeholder-slots';
import { PlayerPlaceholder } from './slots/player-slot';
import { TranscriptSlot } from './slots/transcript-slot';
import type { MeetingPanelSlot, MeetingSlotProps, MeetingTabSlot } from './types';

/** The tabs, in order. */
export const MEETING_TABS: readonly MeetingTabSlot[] = [
  { id: 'notes', label: meetingCopy.notes, shortcut: '⌘1', Component: NotesSlot },
  { id: 'transcript', label: meetingCopy.transcript, shortcut: '⌘2', Component: TranscriptSlot },
  { id: 'insights', label: meetingCopy.insights, shortcut: '⌘3', Component: MeetingInsights },
  { id: 'clips', label: meetingCopy.clips, shortcut: '⌘4', Component: ClipsSlot },
];

/** The right panel's sections, in order. */
export const MEETING_PANELS: readonly MeetingPanelSlot[] = [
  { id: 'chat', label: meetingCopy.chat, Component: ChatPanelSlot },
  { id: 'participants', label: meetingCopy.participants, Component: ParticipantsPanel },
  { id: 'details', label: meetingCopy.details, Component: DetailsPanelSlot },
];

/** The bottom player. */
export const MEETING_PLAYER: ComponentType<MeetingSlotProps> = PlayerPlaceholder;
