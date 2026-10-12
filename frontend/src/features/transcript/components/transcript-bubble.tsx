/**
 * One row of the transcript: a bubble, with its turn's header when it
 * starts one (ui.md Transcript bubble). The user's bubbles sit on the
 * right in the accent tint; everyone else's on the left in the neutral
 * fill, their name in their colour. A line still being spoken is muted
 * with a soft caret, and turns ink in place when it is saved: only the
 * colour changes, so nothing reflows.
 *
 * With a recording in the player (TRN-03), the line being played is in
 * `--highlight` with its current word marked, and a click seeks to the word
 * (or the line) clicked. A double-click edits the text in place (TRN-02).
 */
import type { QueryClient } from '@tanstack/react-query';
import { type KeyboardEvent, type MouseEvent, memo, useMemo } from 'react';

import { playerActions } from '@/features/player';
import { cn } from '@/ui/cn';

import type { TranscriptAction } from '../actions';
import { startEditing, useIsEditing } from '../edit/editing-store';
import { LineEditor } from '../edit/line-editor';
import type { SearchHit } from '../model/search-index';
import { timedWords } from '../model/word-ranges';
import type { MeetingId, TranscriptRow } from '../types';
import { BubbleToolbar } from './bubble-toolbar';
import { HighlightedText } from './highlighted-text';
import { clickedWordStart, SpokenText } from './spoken-text';
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
  /** The line a link pointed at (`aria-current="location"`). */
  marked: boolean;
  /** Its highlight is still showing. */
  flashing: boolean;
  /** Holds the list's keyboard focus. */
  focused: boolean;
  /** The player is on this line. */
  playing: boolean;
  /** The meeting's recording is in the player: clicks seek. */
  playable: boolean;
  /** How many toolbar actions fit beside the bubble. */
  slots: number;
  client: QueryClient;
  actions: readonly TranscriptAction[];
  onFocusRow: (index: number) => void;
  onKeyDown: (event: KeyboardEvent<HTMLDivElement>, index: number) => void;
}

/** The bubble's fill and corner, by side. */
function bubbleClass(isMe: boolean, lit: boolean): string {
  if (lit) return cn('bg-highlight', isMe ? 'rounded-tr-xs' : 'rounded-tl-xs');
  return isMe
    ? 'rounded-tr-xs bg-accent-soft group-hover/bubble:bg-[color-mix(in_srgb,var(--accent-soft),var(--fg)_4%)]'
    : 'rounded-tl-xs bg-bg-muted group-hover/bubble:bg-[color-mix(in_srgb,var(--bg-muted),var(--fg)_4%)]';
}

/** The line's text: search hits, the timed words, or plain. */
function BubbleText({ meetingId, row, hits, current, playing }: TranscriptBubbleProps) {
  const { line } = row;
  const words = useMemo(() => timedWords(line.text, line.words), [line.text, line.words]);
  if (hits !== undefined && hits.length > 0) {
    return <HighlightedText text={line.text} hits={hits} current={current} />;
  }
  if (words === null) return line.text;
  return <SpokenText meetingId={meetingId} text={line.text} words={words} playing={playing} />;
}

/** A click seeks to the word (or line) under it, unless it selected text. */
function seekOnClick(props: TranscriptBubbleProps, event: MouseEvent<HTMLDivElement>): void {
  const { line } = props.row;
  if (!props.playable || line.open || window.getSelection()?.isCollapsed === false) return;
  playerActions.seek(props.meetingId, clickedWordStart(event.target) ?? line.start);
}

/** The bubble's events: focus and keys for the list, click to seek, double-click to edit. */
function bubbleHandlers(props: TranscriptBubbleProps, editing: boolean) {
  const { index, row } = props;
  return {
    onFocus: () => {
      props.onFocusRow(index);
    },
    onKeyDown: (event: KeyboardEvent<HTMLDivElement>) => {
      props.onKeyDown(event, index);
    },
    onClick: (event: MouseEvent<HTMLDivElement>) => {
      if (!editing) seekOnClick(props, event);
    },
    onDoubleClick: () => {
      if (row.line.open || editing) return;
      window.getSelection()?.removeAllRanges();
      startEditing(row.line.id);
    },
  };
}

/** The bubble itself: the line's text (or its editor), and a caret while it is spoken. */
function BubbleBody(props: TranscriptBubbleProps) {
  const { row, index } = props;
  const { line, speaker } = row;
  const editing = useIsEditing(line.id);
  return (
    <div
      role="listitem"
      data-bubble={index}
      data-segment-id={line.open ? undefined : line.id}
      tabIndex={-1}
      aria-busy={line.open || undefined}
      aria-current={props.marked ? 'location' : undefined}
      {...bubbleHandlers(props, editing)}
      className={cn(
        'rounded-lg px-2.5 py-1.5 text-sm text-pretty break-words whitespace-pre-wrap',
        'transition-colors duration-(--dur-slow) ease-out',
        'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus-ring',
        bubbleClass(speaker.isMe, props.flashing || props.playing),
        line.open ? 'text-fg-muted' : 'text-fg',
        editing && 'outline-2 outline-offset-2 outline-focus-ring',
        props.playable && !line.open && !editing && 'cursor-pointer',
      )}
    >
      {editing ? <LineEditor meetingId={props.meetingId} line={line} /> : <BubbleText {...props} />}
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
  // While editing, the bubble takes its full width so the field does not jump.
  const editing = useIsEditing(line.id);
  return (
    <div className={cn(row.turnStart && index > 0 ? 'pt-4' : 'pt-0.5')}>
      {row.turnStart ? <TurnHeader meetingId={props.meetingId} row={row} /> : null}
      <div className={cn('group/bubble flex', speaker.isMe ? 'justify-end' : 'justify-start')}>
        <div className={cn('relative max-w-[75%]', editing && 'w-[75%]')}>
          <BubbleBody {...props} />
          <BubbleToolbar
            context={{
              meetingId: props.meetingId,
              line,
              speaker,
              client: props.client,
              playable: props.playable,
            }}
            actions={props.actions}
            slots={props.slots}
            tabbable={props.focused}
          />
        </div>
      </div>
    </div>
  );
});
