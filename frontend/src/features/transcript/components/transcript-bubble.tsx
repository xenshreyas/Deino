/**
 * One row of the transcript: a bubble, with its turn's header when it
 * starts one (ui.md Transcript bubble). The user's bubbles sit on the
 * right in the accent tint; everyone else's on the left in the neutral
 * fill, their name in their colour. A line still being spoken is muted
 * with a soft caret, and turns ink in place when it is saved: only the
 * colour changes, so nothing reflows.
 */
import { type KeyboardEvent, memo } from 'react';

import { cn } from '@/ui/cn';

import type { TranscriptAction } from '../actions';
import type { SearchHit } from '../model/search-index';
import type { MeetingId, SpeakerClickHandler, TranscriptRow } from '../types';
import { BubbleToolbar } from './bubble-toolbar';
import { HighlightedText } from './highlighted-text';
import { TurnHeader } from './turn-header';

/** Props of {@link TranscriptBubble}. */
export interface TranscriptBubbleProps {
  meetingId: MeetingId;
  row: TranscriptRow;
  /** The row's position, for focus and keyboard moves. */
  index: number;
  hits: readonly SearchHit[] | undefined;
  /** The current hit's ordinal, when it is in this row. */
  current: number | undefined;
  /** Briefly marked: the moment a link pointed at. */
  flashing: boolean;
  /** Holds the list's keyboard focus. */
  focused: boolean;
  actions: readonly TranscriptAction[];
  onSpeakerClick: SpeakerClickHandler | undefined;
  onFocusRow: (index: number) => void;
  onKeyDown: (event: KeyboardEvent<HTMLDivElement>, index: number) => void;
}

/** The bubble's fill and corner, by side. */
function bubbleClass(isMe: boolean, flashing: boolean): string {
  if (flashing) return cn('bg-highlight', isMe ? 'rounded-tr-xs' : 'rounded-tl-xs');
  return isMe
    ? 'rounded-tr-xs bg-accent-soft group-hover/bubble:bg-[color-mix(in_srgb,var(--accent-soft),var(--fg)_4%)]'
    : 'rounded-tl-xs bg-bg-muted group-hover/bubble:bg-[color-mix(in_srgb,var(--bg-muted),var(--fg)_4%)]';
}

/** The bubble itself: the line's text, its hits, and a caret while it is spoken. */
function BubbleBody(props: TranscriptBubbleProps) {
  const { row, index } = props;
  const { line, speaker } = row;
  return (
    <div
      role="listitem"
      data-bubble={index}
      tabIndex={-1}
      aria-busy={line.open || undefined}
      aria-current={props.flashing ? 'location' : undefined}
      onFocus={() => {
        props.onFocusRow(index);
      }}
      onKeyDown={(event) => {
        props.onKeyDown(event, index);
      }}
      className={cn(
        'rounded-lg px-2.5 py-1.5 text-sm text-pretty break-words whitespace-pre-wrap',
        'transition-colors duration-(--dur-slow) ease-out',
        'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus-ring',
        bubbleClass(speaker.isMe, props.flashing),
        line.open ? 'text-fg-muted' : 'text-fg',
      )}
    >
      <HighlightedText text={line.text} hits={props.hits} current={props.current} />
      {line.open ? (
        <span
          aria-hidden
          className="ml-0.5 inline-block h-3.5 w-0.5 translate-y-0.5 animate-pulse-soft rounded-full bg-accent"
        />
      ) : null}
    </div>
  );
}

/** A row; memoised so a growing live line re-renders only itself. */
export const TranscriptBubble = memo(function TranscriptBubble(props: TranscriptBubbleProps) {
  const { row, index } = props;
  const { line, speaker } = row;
  return (
    <div className={cn(row.turnStart && index > 0 ? 'pt-4' : 'pt-0.5')}>
      {row.turnStart ? (
        <TurnHeader speaker={speaker} start={line.start} onSpeakerClick={props.onSpeakerClick} />
      ) : null}
      <div className={cn('group/bubble flex', speaker.isMe ? 'justify-end' : 'justify-start')}>
        <div className="relative max-w-[75%]">
          <BubbleBody {...props} />
          <BubbleToolbar
            context={{ meetingId: props.meetingId, line, speaker }}
            actions={props.actions}
            tabbable={props.focused}
          />
        </div>
      </div>
    </div>
  );
});
