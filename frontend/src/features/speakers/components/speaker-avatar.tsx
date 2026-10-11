/**
 * A speaker's avatar (ui.md §3 Speaker chip): initials in the speaker's
 * colour on that colour at 16%. An unnamed speaker shows a dashed outline
 * instead, so "who is this?" reads at a glance; a person outside the
 * meeting (in the name picker) is a quiet grey disc.
 */
import { cn } from '@/ui/cn';

import { initials, speakerTone } from '../format';

/** Props of {@link SpeakerAvatar}. */
export interface SpeakerAvatarProps {
  name: string;
  /** The `--speaker-N` slot; grey without one. */
  color?: number | undefined;
  /** Nobody is assigned yet: a dashed, uncoloured ring. */
  unnamed?: boolean;
  /** 24 in lists, 14 inline in chips. */
  size?: 14 | 24;
  className?: string;
}

/** The look of an avatar. */
function look(color: number | undefined, unnamed: boolean): string {
  if (unnamed) return 'border border-dashed border-border-strong text-fg-muted';
  if (color === undefined) return 'bg-bg-muted text-fg-muted';
  const tone = speakerTone(color);
  return cn(tone.soft, tone.text);
}

/** The round avatar with initials. */
export function SpeakerAvatar({
  name,
  color,
  unnamed = false,
  size = 24,
  className,
}: SpeakerAvatarProps) {
  return (
    <span
      aria-hidden
      className={cn(
        'inline-flex shrink-0 items-center justify-center rounded-full font-semibold tabular-nums select-none',
        size === 24 ? 'size-6 text-[10px]' : 'size-3.5 text-[9px]',
        look(color, unnamed),
        className,
      )}
    >
      {initials(name)}
    </span>
  );
}
