/**
 * The status row while notes are written (streamed notes): what the run is
 * doing ("Writing · Discussion"), Stop, the quiet hint that editing waits
 * for the saved notes, and the note a finished or stopped run leaves.
 */
import { useState } from 'react';

import type { MeetingId } from '@/features/meetings';
import { useMountEffect } from '@/hooks/useMountEffect';
import { Button } from '@/ui/button';
import { Check, ICON_PROPS, Sparkles, Square } from '@/ui/icons';
import { Hint } from '@/ui/tooltip';

import { notesCopy } from '../copy';
import { type LiveNotes, useForgetRun, useStopNotes } from '../hooks/use-live-notes';
import type { Block, NotesDraft } from '../types';

/** How long a finished or stopped run's note stays, in ms. */
const NOTE_MS = 5000;

/** The text of a top-level heading. */
function headingText(block: Block): string | null {
  if (block.type !== 'heading') return null;
  return (block.content ?? [])
    .map((inline) => (inline.type === 'text' ? inline.text : ''))
    .join('');
}

/** What the run is doing, in a few words. */
export function runStatusText(draft: NotesDraft): string {
  const { phase } = draft;
  const shown = draft.lines > 0 && !draft.stale;
  const headings = draft.doc.content.flatMap((block) => headingText(block) ?? []);
  const heading = headings.at(-1) ?? '';
  switch (phase.kind) {
    case 'parts':
      return notesCopy.readingParts(phase.done, phase.total);
    case 'retrying':
      return notesCopy.retrying;
    case 'saving':
      return notesCopy.saving;
    case 'merging':
      return shown ? notesCopy.writingSection(heading) : notesCopy.merging;
    case 'reading':
      return shown ? notesCopy.writingSection(heading) : notesCopy.readingTranscript;
  }
}

/** "Writing · Discussion", announced politely. */
export function RunStatus({ draft }: { draft: NotesDraft }) {
  return (
    <p
      role="status"
      aria-live="polite"
      className="flex min-w-0 items-center gap-2 text-sm text-ai-text"
    >
      <Sparkles {...ICON_PROPS} className="shrink-0 animate-pulse text-accent" />
      <span className="truncate">{runStatusText(draft)}</span>
    </p>
  );
}

/** Stop, while the notes are still being written (not while they save). */
export function StopButton({ meetingId, draft }: { meetingId: MeetingId; draft: NotesDraft }) {
  const stop = useStopNotes(meetingId);
  if (draft.phase.kind === 'saving') return null;
  return (
    <Hint label={notesCopy.stopHint}>
      <Button
        variant="ghost"
        size="sm"
        onClick={() => {
          stop.mutate();
        }}
        disabled={stop.isPending}
      >
        <Square {...ICON_PROPS} />
        {notesCopy.stop}
      </Button>
    </Hint>
  );
}

/** "You can edit once the notes are written", for a moment after a click. */
export function useLockedHint() {
  const [shown, setShown] = useState(0);
  return {
    hint:
      shown === 0 ? null : (
        <LockedHint
          key={shown}
          onDone={() => {
            setShown(0);
          }}
        />
      ),
    flash: () => {
      setShown((count) => count + 1);
    },
  };
}

function LockedHint({ onDone }: { onDone: () => void }) {
  useMountEffect(() => {
    const timer = window.setTimeout(onDone, 2500);
    return () => {
      window.clearTimeout(timer);
    };
  });
  return <span className="text-xs whitespace-nowrap text-fg-subtle">{notesCopy.lockedHint}</span>;
}

/** The note a finished or stopped run leaves, for a few seconds. */
export function RunNote(props: {
  meetingId: MeetingId;
  live: LiveNotes | null;
  /** The version on the page, if any. */
  version: number | null;
}) {
  const { live, version } = props;
  if (live === null || live.kind === 'writing') return null;
  if (live.kind === 'written' && (version ?? 0) < live.version) return null;
  const text =
    live.kind === 'written'
      ? notesCopy.written(live.kept)
      : version === null
        ? notesCopy.stoppedNothing
        : notesCopy.stoppedKept(version);
  return (
    <FadingNote key={live.kind} meetingId={props.meetingId} done={live.kind === 'written'}>
      {text}
    </FadingNote>
  );
}

function FadingNote(props: { meetingId: MeetingId; done: boolean; children: string }) {
  const forget = useForgetRun(props.meetingId);
  useMountEffect(() => {
    const timer = window.setTimeout(forget, NOTE_MS);
    return () => {
      window.clearTimeout(timer);
    };
  });
  return (
    <span role="status" className="flex min-w-0 items-center gap-1.5 text-xs text-fg-muted">
      {props.done ? <Check {...ICON_PROPS} size={14} className="shrink-0 text-success" /> : null}
      <span className="truncate">{props.children}</span>
    </span>
  );
}
