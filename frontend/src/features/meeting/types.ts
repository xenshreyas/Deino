/** Types of the meeting page and its slot registry. */
import type { ComponentType } from 'react';

import type { MeetingId } from '@/features/meetings';
import type { TranscriptMoment } from '@/features/transcript';

import type { MeetingTabId } from './search';

export type { MeetingTabId } from './search';

/** The right panel's sections. */
export type MeetingPanelId = 'chat' | 'details';

/** A moment a link asked to show (`?t=` or `?segment=`). */
export interface MeetingMoment {
  /** A time of the recording, or one line. */
  target: TranscriptMoment;
  /** Called once it has been shown, to drop it from the URL. */
  onShown: () => void;
}

/** What every slot component receives. */
export interface MeetingSlotProps {
  meetingId: MeetingId;
  /** The moment to show, for the slots that can (the Transcript tab). */
  moment?: MeetingMoment | undefined;
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
