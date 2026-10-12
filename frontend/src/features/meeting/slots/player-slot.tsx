/**
 * The bottom player slot (TRN-03): the meeting's recording in the docked
 * player, its scrubber painted with who spoke when from the transcript, and
 * seeking to a linked moment (`?t=`). Nothing shows for a meeting without a
 * recording on this Mac.
 */
import { MeetingPlayer } from '@/features/player';
import { useSpeakerBands } from '@/features/transcript';

import type { MeetingSlotProps } from '../types';

/** The slot. */
export function PlayerSlot({ meetingId, moment }: MeetingSlotProps) {
  const bands = useSpeakerBands(meetingId);
  const target = moment?.target;
  return (
    <MeetingPlayer
      meetingId={meetingId}
      bands={bands}
      seekTo={target?.kind === 'time' ? target.at : undefined}
    />
  );
}
