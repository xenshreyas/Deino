/**
 * The comment popovers, anchored where the text is: the composer on a
 * selection, and a thread opened from its highlight.
 */
import type { ReactNode } from 'react';

import { Popover, PopoverAnchor, PopoverContent } from '@/ui/popover';

import { commentsCopy } from '../copy';
import { useStartThread, useThreads } from '../queries';
import { closeCommentPopover, useCommentsUi } from '../store';
import type { MeetingId, ScreenRect } from '../types';
import { CommentBox } from './comment-box';
import { ThreadCard } from './thread-card';

/** A popover under `rect`. */
function AnchoredPopover({
  rect,
  label,
  children,
}: {
  rect: ScreenRect;
  label: string;
  children: ReactNode;
}) {
  return (
    <Popover
      open
      onOpenChange={(open) => {
        if (!open) closeCommentPopover();
      }}
    >
      <PopoverAnchor asChild>
        <div aria-hidden className="pointer-events-none fixed" style={{ ...rect }} />
      </PopoverAnchor>
      <PopoverContent aria-label={label} align="start" className="w-80">
        {children}
      </PopoverContent>
    </Popover>
  );
}

/** The composer of a new thread, while one is being written. */
function Composer() {
  const draft = useCommentsUi((state) => state.draft);
  const start = useStartThread();
  if (draft === null) return null;
  const submit = async (body: string) => {
    await start.mutateAsync({ meetingId: draft.meetingId, anchor: draft.anchor, body });
    closeCommentPopover();
  };
  return (
    <AnchoredPopover rect={draft.rect} label={commentsCopy.comment}>
      <blockquote className="mb-2 line-clamp-2 border-l-2 border-warning pl-2 text-xs text-fg-muted">
        {draft.anchor.quote}
      </blockquote>
      <CommentBox
        autoFocus
        placeholder={commentsCopy.placeholder}
        submitLabel={commentsCopy.send}
        busy={start.isPending}
        onSubmit={submit}
        onCancel={closeCommentPopover}
      />
    </AnchoredPopover>
  );
}

/** The thread opened from its highlight. */
function OpenedThread({ meetingId }: { meetingId: MeetingId }) {
  const open = useCommentsUi((state) => state.open);
  const { data: threads = [] } = useThreads(meetingId);
  const thread = open === null ? undefined : threads.find(({ id }) => id === open.threadId);
  if (open === null || thread === undefined) return null;
  return (
    <AnchoredPopover rect={open.rect} label={commentsCopy.comment}>
      <ThreadCard thread={thread} detached={false} autoFocus />
    </AnchoredPopover>
  );
}

/** Both popovers of a meeting page. */
export function CommentPopovers({ meetingId }: { meetingId: MeetingId }) {
  return (
    <>
      <Composer />
      <OpenedThread meetingId={meetingId} />
    </>
  );
}
