/**
 * A turn's speaker chip (ui.md Speaker chip): CAP-04's 14px avatar and the
 * name in the speaker's colour, muted with a dashed avatar while nobody is
 * assigned. Clicking it opens CAP-04's "Who is …?" popover for the turn's
 * first line, which renames "This segment" or "All segments". A live line
 * not saved yet has no stored speaker, so its chip is plain text.
 */
import { useState } from 'react';

import { AssignSpeaker, SpeakerAvatar, speakerTone } from '@/features/speakers';
import { cn } from '@/ui/cn';
import { Popover, PopoverContent, PopoverTrigger } from '@/ui/popover';

import type { MeetingId, SegmentId, SpeakerView } from '../types';

/** Props of {@link SpeakerChip}. */
export interface SpeakerChipProps {
  meetingId: MeetingId;
  speaker: SpeakerView;
  /** The line the chip heads; `null` while it is still being spoken. */
  segmentId: SegmentId | null;
}

/** The avatar and the name. */
function ChipFace({ speaker }: { speaker: SpeakerView }) {
  return (
    <>
      <SpeakerAvatar
        name={speaker.name}
        color={speaker.color}
        unnamed={speaker.unnamed}
        size={14}
      />
      <span
        className={cn(
          'truncate text-xs font-semibold',
          speaker.unnamed ? 'text-fg-muted' : speakerTone(speaker.color).text,
        )}
      >
        {speaker.name}
      </span>
    </>
  );
}

/** The chip, with its "Who is …?" popover when the speaker can be named. */
export function SpeakerChip({ meetingId, speaker, segmentId }: SpeakerChipProps) {
  const [open, setOpen] = useState(false);
  const { speakerId } = speaker;
  if (speakerId === null || segmentId === null) {
    return (
      <span className="flex min-w-0 items-center gap-1.5">
        <ChipFace speaker={speaker} />
      </span>
    );
  }
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger className="-mx-1 flex min-w-0 items-center gap-1.5 rounded-sm px-1 transition-colors duration-(--dur-fast) ease-out hover:bg-bg-muted focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus-ring data-[state=open]:bg-bg-muted">
        <ChipFace speaker={speaker} />
      </PopoverTrigger>
      <PopoverContent align={speaker.isMe ? 'end' : 'start'} className="w-72 p-0">
        <AssignSpeaker
          meetingId={meetingId}
          speakerId={speakerId}
          name={speaker.name}
          segmentId={segmentId}
          onDone={() => {
            setOpen(false);
          }}
        />
      </PopoverContent>
    </Popover>
  );
}
