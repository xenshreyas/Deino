/**
 * The meeting's Comments panel (TRN-06): every open thread, oldest first,
 * each under where it is (Transcript, My notes, Notes); resolved threads
 * fold away behind "Show n resolved". A transcript thread whose words are
 * gone is marked "Detached", never dropped.
 */
import { useState } from 'react';

import { Button } from '@/ui/button';
import { ICON_PROPS_LARGE, MessageCircle } from '@/ui/icons';

import { commentsCopy } from '../copy';
import { locateQuote } from '../model/anchor';
import { useThreads } from '../queries';
import type { CommentTarget, CommentThread, MeetingId } from '../types';
import { ThreadCard } from './thread-card';

/** Where a thread is, as its label. */
const PLACE: Record<CommentTarget, string> = {
  transcript: commentsCopy.inTranscript,
  notes: commentsCopy.inNotes,
  enhanced: commentsCopy.inEnhanced,
};

/** Whether a transcript thread's words can no longer be found. */
export function isDetached(thread: CommentThread, lines: ReadonlyMap<string, string>): boolean {
  if (thread.anchor.target !== 'transcript') return false;
  const text = thread.anchor.segmentId === null ? undefined : lines.get(thread.anchor.segmentId);
  return text === undefined || locateQuote(text, thread.anchor) === null;
}

/** The resting state. */
function Empty() {
  return (
    <div className="flex flex-col items-start gap-2 px-4 py-6">
      <div className="flex size-9 items-center justify-center rounded-lg bg-bg-muted text-fg-muted">
        <MessageCircle {...ICON_PROPS_LARGE} size={18} />
      </div>
      <h3 className="pt-1 text-sm font-semibold text-fg">{commentsCopy.emptyTitle}</h3>
      <p className="text-sm text-pretty text-fg-muted">{commentsCopy.emptyBody}</p>
    </div>
  );
}

/** A list of threads. */
function Threads({
  threads,
  lines,
}: {
  threads: readonly CommentThread[];
  lines: ReadonlyMap<string, string> | null;
}) {
  return (
    <ul className="flex flex-col divide-y divide-border">
      {threads.map((thread) => (
        <li key={thread.id} className="flex flex-col gap-2 px-4 py-3">
          <span className="text-xs font-semibold text-fg-muted">{PLACE[thread.anchor.target]}</span>
          <ThreadCard thread={thread} detached={lines !== null && isDetached(thread, lines)} />
        </li>
      ))}
    </ul>
  );
}

/** Props of {@link CommentsPanel}. */
export interface CommentsPanelProps {
  meetingId: MeetingId;
  /** The transcript's lines' texts by ID (deleted lines absent); `null` while loading. */
  lineTexts: ReadonlyMap<string, string> | null;
}

/** The panel. */
export function CommentsPanel({ meetingId, lineTexts: lines }: CommentsPanelProps) {
  const { data: threads = [] } = useThreads(meetingId);
  const [showResolved, setShowResolved] = useState(false);
  if (threads.length === 0) return <Empty />;
  const open = threads.filter((thread) => thread.resolvedAt === null);
  const resolved = threads.filter((thread) => thread.resolvedAt !== null);
  return (
    <div className="flex flex-col">
      <Threads threads={open} lines={lines} />
      {resolved.length === 0 ? null : (
        <div className="px-4 py-2">
          <Button
            size="sm"
            variant="ghost"
            onClick={() => {
              setShowResolved((shown) => !shown);
            }}
          >
            {showResolved ? commentsCopy.hideResolved : commentsCopy.showResolved(resolved.length)}
          </Button>
        </div>
      )}
      {showResolved ? <Threads threads={resolved} lines={lines} /> : null}
    </div>
  );
}
