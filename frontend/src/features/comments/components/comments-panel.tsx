/**
 * The meeting's comments, a section of its Details panel (TRN-06, UX-04):
 * every open thread, oldest first, each under where it is (Transcript, My
 * notes, Notes); resolved threads fold away behind "Show n resolved". A
 * transcript thread whose words are gone is marked "Detached", never
 * dropped. Nothing shows until the meeting has a comment.
 */
import { useState } from 'react';

import { Button } from '@/ui/button';
import { PanelSection } from '@/ui/panel-section';

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
        <li key={thread.id} className="flex flex-col gap-2 py-3">
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

/** The section, or nothing while the meeting has no comments. */
export function CommentsPanel({ meetingId, lineTexts: lines }: CommentsPanelProps) {
  const { data: threads = [] } = useThreads(meetingId);
  const [showResolved, setShowResolved] = useState(false);
  if (threads.length === 0) return null;
  const open = threads.filter((thread) => thread.resolvedAt === null);
  const resolved = threads.filter((thread) => thread.resolvedAt !== null);
  return (
    <PanelSection title={commentsCopy.panel}>
      <Threads threads={open} lines={lines} />
      {resolved.length === 0 ? null : (
        <div className="py-1">
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
    </PanelSection>
  );
}
