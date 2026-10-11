/**
 * A speaker turn's header, 4px above its first bubble (ui.md Transcript
 * bubble, Speaker chip): a 14px avatar and the name in the speaker's
 * colour, and the time. The time sits right of the name for others and
 * left of it for the user. The user's voice takes the accent, the colour
 * of "mine"; a voice nobody has named yet is muted, with a dashed avatar.
 */
import { formatElapsed } from '@/features/recording';
import { cn } from '@/ui/cn';

import { initials, speakerTextClass, speakerTintClass } from '../model/speakers';
import type { SpeakerClickHandler, SpeakerView } from '../types';

/** The name's colour classes. */
function nameClass(speaker: SpeakerView): string {
  if (speaker.isMe) return 'text-accent';
  return speaker.identified ? speakerTextClass(speaker.color) : 'text-fg-muted';
}

/** The avatar's fill and border classes. */
function avatarClass(speaker: SpeakerView): string {
  if (speaker.isMe) return 'bg-accent-soft text-accent';
  if (!speaker.identified) return 'border border-dashed border-fg-subtle text-fg-muted';
  return cn(speakerTintClass(speaker.color), speakerTextClass(speaker.color));
}

/** The avatar and name; a button when the speaker can be clicked (CAP-04). */
function SpeakerChip({
  speaker,
  onClick,
}: {
  speaker: SpeakerView;
  onClick: SpeakerClickHandler | undefined;
}) {
  const content = (
    <>
      <span
        aria-hidden
        className={cn(
          'flex size-3.5 shrink-0 items-center justify-center rounded-full text-[9px] leading-none font-semibold',
          avatarClass(speaker),
        )}
      >
        {initials(speaker.name)}
      </span>
      <span className={cn('truncate text-xs font-semibold', nameClass(speaker))}>
        {speaker.name}
      </span>
    </>
  );
  if (onClick === undefined)
    return <span className="flex min-w-0 items-center gap-1.5">{content}</span>;
  return (
    <button
      type="button"
      onClick={(event) => {
        onClick(speaker, event.currentTarget);
      }}
      className="-mx-1 flex min-w-0 items-center gap-1.5 rounded-sm px-1 transition-colors duration-(--dur-fast) ease-out hover:bg-bg-muted focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus-ring"
    >
      {content}
    </button>
  );
}

/** Props of {@link TurnHeader}. */
export interface TurnHeaderProps {
  speaker: SpeakerView;
  /** Milliseconds from the start of the recording. */
  start: number;
  onSpeakerClick: SpeakerClickHandler | undefined;
}

/** The header of a turn. */
export function TurnHeader({ speaker, start, onSpeakerClick }: TurnHeaderProps) {
  const seconds = Math.floor(start / 1000);
  return (
    <div className={cn('flex h-5 items-center gap-2 pb-1', speaker.isMe && 'flex-row-reverse')}>
      <SpeakerChip speaker={speaker} onClick={onSpeakerClick} />
      <time dateTime={`PT${seconds}S`} className="shrink-0 text-xs text-fg-subtle tabular-nums">
        {formatElapsed(seconds)}
      </time>
    </div>
  );
}
