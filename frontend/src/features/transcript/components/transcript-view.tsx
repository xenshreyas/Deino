/**
 * The transcript view (TRN-01): the search bar over the virtual list of
 * speaker turns, "Jump to now" while live, and a deep link's moment.
 * Presentational: rows and speakers come in, actions and speaker clicks go
 * out; `TranscriptPanel` feeds it a meeting's transcript.
 */
import { type ReactNode, useState } from 'react';

import { TRANSCRIPT_ACTIONS, type TranscriptAction } from '../actions';
import { useTranscriptList } from '../hooks/use-transcript-list';
import { useTranscriptSearch } from '../hooks/use-transcript-search';
import type { MeetingId, SpeakerClickHandler, SpeakerView, TranscriptRow } from '../types';
import { JumpToNow } from './jump-to-now';
import { MomentFocus } from './moment-focus';
import { SearchBar } from './search-bar';
import { TranscriptList } from './transcript-list';

/** Props of {@link TranscriptView}. */
export interface TranscriptViewProps {
  meetingId: MeetingId;
  rows: readonly TranscriptRow[];
  speakers: readonly SpeakerView[];
  /** Being recorded: follow the newest line, offer "Jump to now". */
  live: boolean;
  /** A moment to show on mount, in milliseconds (`?t=`). */
  focusAt?: number | undefined;
  onFocusDone?: (() => void) | undefined;
  /** Clicking a speaker's name (CAP-04's rename popover); plain text without it. */
  onSpeakerClick?: SpeakerClickHandler | undefined;
  /** The bubble actions; the registry by default. */
  actions?: readonly TranscriptAction[];
  /** Tools beside the search field. */
  tools?: ReactNode;
}

/** The view. */
export function TranscriptView(props: TranscriptViewProps) {
  const { rows, speakers, live, focusAt } = props;
  const list = useTranscriptList(rows, live);
  const search = useTranscriptSearch(rows, speakers, list.reveal);
  const [flashing, setFlashing] = useState<string | null>(null);
  return (
    <div className="relative flex h-full min-h-0 flex-col">
      <SearchBar search={search} speakers={speakers} tools={props.tools} />
      <TranscriptList
        meetingId={props.meetingId}
        rows={rows}
        list={list}
        result={search.result}
        current={search.current}
        flashing={flashing}
        actions={props.actions ?? TRANSCRIPT_ACTIONS}
        onSpeakerClick={props.onSpeakerClick}
      />
      {live ? <JumpToNow visible={!list.following} onJump={list.jumpToNow} /> : null}
      {focusAt === undefined ? null : (
        <MomentFocus
          key={focusAt}
          at={focusAt}
          rows={rows}
          reveal={list.reveal}
          onFlash={setFlashing}
          onDone={props.onFocusDone}
        />
      )}
    </div>
  );
}
