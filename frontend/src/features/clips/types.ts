/** Types of clips and moments: the generated ones and the clip being named. */
export type { Clip, ClipId, ClipMedia, MeetingId, NewClip } from '@/lib/ipc/generated/clips';

import type { MeetingId } from '@/lib/ipc/generated/clips';

/** A range the user chose to clip, waiting for its name. */
export interface ClipDraft {
  meetingId: MeetingId;
  /** Milliseconds from the start of the recording. */
  start: number;
  /** Milliseconds from the start of the recording, after `start`. */
  end: number;
  /** The name the dialog suggests: the selected words. */
  suggestedName: string;
}
