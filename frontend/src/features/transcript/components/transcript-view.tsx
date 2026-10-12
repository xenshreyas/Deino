/**
 * The transcript view (TRN-01): the search bar over the virtual list of
 * speaker turns, "Jump to now" while live, and a deep link's moment. A saved
 * transcript adds the TRN-02 tools: the replace row under the search bar
 * (toggled beside it, or ⌥⌘F) and "Transcript changed: Regenerate notes?"
 * after an edit. Presentational apart from those: rows and speakers come
 * in, actions and speaker clicks go out; `TranscriptPanel` feeds it.
 */
import { type ReactNode, useState } from 'react';

import { TRANSCRIPT_ACTIONS, type TranscriptAction } from '../actions';
import { ReplaceBar } from '../edit/replace-bar';
import { type ReplaceToggle, ReplaceToggleButton, useReplaceToggle } from '../edit/replace-toggle';
import { TranscriptChangedBanner } from '../edit/transcript-changed-banner';
import { useTranscriptList } from '../hooks/use-transcript-list';
import { useTranscriptSearch } from '../hooks/use-transcript-search';
import type { MeetingId, SpeakerView, TranscriptMoment, TranscriptRow } from '../types';
import { JumpToNow } from './jump-to-now';
import { MomentFocus, type MomentMark, momentKey } from './moment-focus';
import { SearchBar } from './search-bar';
import { TranscriptList } from './transcript-list';

/** Props of {@link TranscriptView}. */
export interface TranscriptViewProps {
  meetingId: MeetingId;
  rows: readonly TranscriptRow[];
  speakers: readonly SpeakerView[];
  /** Being recorded: follow the newest line, offer "Jump to now". */
  live: boolean;
  /** A moment to show (`?t=`, `?segment=`); a new one scrolls there again. */
  moment?: TranscriptMoment | undefined;
  /** Called once the moment has been shown. */
  onMomentShown?: (() => void) | undefined;
  /** The bubble actions; the registry by default. */
  actions?: readonly TranscriptAction[];
  /** Tools beside the search field. */
  tools?: ReactNode;
}

/** The search bar's tools with the replace toggle first. */
function withReplace(replace: ReplaceToggle, tools: ReactNode): ReactNode {
  return (
    <>
      <ReplaceToggleButton {...replace} />
      {tools}
    </>
  );
}

/** The view. */
export function TranscriptView(props: TranscriptViewProps) {
  const { rows, speakers, live, moment, meetingId } = props;
  const list = useTranscriptList(rows, live);
  const search = useTranscriptSearch(rows, speakers, list.reveal);
  const [mark, setMark] = useState<MomentMark | null>(null);
  const replace = useReplaceToggle(live);
  const tools = live ? props.tools : withReplace(replace, props.tools);
  return (
    <div className="relative flex h-full min-h-0 flex-col">
      <SearchBar search={search} speakers={speakers} tools={tools} />
      {replace.open ? <ReplaceBar meetingId={meetingId} find={search.query.text} /> : null}
      {live ? null : <TranscriptChangedBanner meetingId={meetingId} />}
      <TranscriptList
        meetingId={meetingId}
        rows={rows}
        list={list}
        result={search.result}
        current={search.current}
        mark={mark}
        actions={props.actions ?? TRANSCRIPT_ACTIONS}
      />
      {live ? <JumpToNow visible={!list.following} onJump={list.jumpToNow} /> : null}
      {moment === undefined ? null : (
        <MomentFocus
          key={momentKey(moment)}
          moment={moment}
          rows={rows}
          reveal={list.reveal}
          onMark={setMark}
          onDone={props.onMomentShown}
        />
      )}
    </div>
  );
}
