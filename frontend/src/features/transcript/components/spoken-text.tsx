/**
 * A line's text as timed words (TRN-03): each word a span carrying its
 * start (`data-at`), so a click seeks there. The line being played is in
 * `--highlight` (ui.md), the word being said a shade deeper. Only the
 * playing line subscribes to the position, so the rest of the list never
 * re-renders as it plays.
 */
import type { ReactNode } from 'react';

import { usePlaybackValue } from '@/features/player';
import { cn } from '@/ui/cn';

import { type TimedWord, wordAt } from '../model/word-ranges';
import type { MeetingId } from '../types';

/** Props of {@link SpokenText}. */
export interface SpokenTextProps {
  meetingId: MeetingId;
  text: string;
  words: readonly TimedWord[];
  /** This line is being played: follow the current word. */
  playing: boolean;
}

/** The words as spans, the one at `current` marked. */
function Words({
  text,
  words,
  current,
}: {
  text: string;
  words: readonly TimedWord[];
  current: number;
}) {
  const parts: ReactNode[] = [];
  let cursor = 0;
  for (const [index, word] of words.entries()) {
    parts.push(text.slice(cursor, word.range.start));
    parts.push(
      <span
        key={index}
        data-at={word.start}
        className={cn(
          'rounded-xs transition-colors duration-(--dur-fast) ease-out',
          index === current && 'bg-[color-mix(in_srgb,var(--highlight),var(--fg)_12%)]',
        )}
      >
        {text.slice(word.range.start, word.range.end)}
      </span>,
    );
    cursor = word.range.end;
  }
  parts.push(text.slice(cursor));
  return <>{parts}</>;
}

/** The playing line's words, following the position. */
function PlayingWords(props: SpokenTextProps) {
  const current = usePlaybackValue(props.meetingId, (ms) => wordAt(props.words, ms)) ?? -1;
  return <Words text={props.text} words={props.words} current={current} />;
}

/** The text; the current word marked while the line plays. */
export function SpokenText(props: SpokenTextProps) {
  return props.playing ? (
    <PlayingWords {...props} />
  ) : (
    <Words text={props.text} words={props.words} current={-1} />
  );
}

/** The time a click on `target` asks for: its word's start, if it is a word. */
export function clickedWordStart(target: EventTarget | null): number | undefined {
  if (!(target instanceof HTMLElement)) return undefined;
  const at = target.closest<HTMLElement>('[data-at]')?.dataset['at'];
  const ms = at === undefined ? Number.NaN : Number(at);
  return Number.isFinite(ms) ? ms : undefined;
}
