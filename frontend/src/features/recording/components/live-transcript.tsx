/**
 * The live transcript beside the notes: one line per segment, labelled Me
 * (microphone) or Them (system audio), newest at the bottom. The list is
 * laid out bottom-up, so it stays scrolled to the newest line without
 * scripting.
 */
import type { MeetingId } from '@/features/meetings';
import { cn } from '@/ui/cn';

import { recordingCopy } from '../copy';
import { formatElapsed } from '../elapsed';
import { useLiveLines } from '../hooks/use-live-lines';
import type { Channel, TranscriptLine } from '../types';

/** The label of a line's channel. */
function speaker(channel: Channel): string {
  return channel === 'mic' ? recordingCopy.me : recordingCopy.them;
}

function Line({ line }: { line: TranscriptLine }) {
  return (
    <li className="flex flex-col gap-0.5">
      <div className="flex items-baseline gap-2 text-xs">
        <span className={cn('font-medium', line.channel === 'mic' ? 'text-accent' : 'text-fg')}>
          {speaker(line.channel)}
        </span>
        <span className="text-fg-subtle tabular-nums">{formatElapsed(line.start / 1000)}</span>
      </div>
      <p className={cn('text-sm leading-relaxed', line.open ? 'text-fg-muted' : 'text-fg')}>
        {line.text}
      </p>
    </li>
  );
}

/** The lines of the recording of `meetingId`. */
export function LiveTranscript({ meetingId }: { meetingId: MeetingId }) {
  const lines = useLiveLines(meetingId);
  if (lines.length === 0) {
    return <p className="px-4 py-6 text-sm text-fg-muted">{recordingCopy.transcriptEmpty}</p>;
  }
  return (
    <div className="flex min-h-0 flex-1 flex-col-reverse overflow-y-auto px-4 py-4">
      <ol aria-label={recordingCopy.transcriptHeading} className="flex flex-col gap-3">
        {lines.map((line) => (
          <Line key={line.id} line={line} />
        ))}
      </ol>
    </div>
  );
}
