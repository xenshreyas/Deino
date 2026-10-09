/** Types of the meeting page and its slot registry. */
import type { ComponentType } from 'react';

import type { MeetingId } from '@/features/meetings';

import type { MeetingTabId } from './search';

export type { MeetingTabId } from './search';

/** The right panel's sections. */
export type MeetingPanelId = 'chat' | 'participants' | 'details';

/** What every slot component receives. */
export interface MeetingSlotProps {
  meetingId: MeetingId;
}

/** A tab of the meeting page. */
export interface MeetingTabSlot {
  id: MeetingTabId;
  label: string;
  /** The ⌘-number that selects it (`⌘1` …). */
  shortcut: string;
  Component: ComponentType<MeetingSlotProps>;
}

/** A section of the right panel. */
export interface MeetingPanelSlot {
  id: MeetingPanelId;
  label: string;
  Component: ComponentType<MeetingSlotProps>;
}
