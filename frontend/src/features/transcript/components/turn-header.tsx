/**
 * A speaker turn's header, 4px above its first bubble (ui.md Transcript
 * bubble): the speaker chip and the time. The time sits right of the name
 * for others and left of it for the user.
 */
import { formatElapsed } from '@/features/recording';
import { cn } from '@/ui/cn';

import type { MeetingId, TranscriptLine, TranscriptRow } from '../types';
import { SpeakerChip } from './speaker-chip';

/** Props of {@link TurnHeader}. */
export interface TurnHeaderProps {
  meetingId: MeetingId;
  row: TranscriptRow;
}

/** The ID a rename of "this segment" can use: none while the line is still spoken. */
function savedId(line: TranscriptLine) {
  return line.open ? null : line.id;
}

/** The header of a turn. */
export function TurnHeader({ meetingId, row }: TurnHeaderProps) {
  const { speaker, line } = row;
  const seconds = Math.floor(line.start / 1000);
  return (
    <div className={cn('flex h-5 items-center gap-2 pb-1', speaker.isMe && 'flex-row-reverse')}>
      <SpeakerChip meetingId={meetingId} speaker={speaker} segmentId={savedId(line)} />
      <time
        dateTime={`PT${String(seconds)}S`}
        className="shrink-0 text-xs text-fg-subtle tabular-nums"
      >
        {formatElapsed(seconds)}
      </time>
    </div>
  );
}
