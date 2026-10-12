/**
 * One thread: the quoted span (marked "Detached" when it can no longer be
 * found), its comments with their times, a reply field, and Resolve.
 */
import { formatDistanceToNowStrict } from 'date-fns';

import { CircleCheck, ICON_PROPS, RotateCcw, Trash2 } from '@/ui/icons';
import { IconButton } from '@/ui/icon-button';
import { Hint } from '@/ui/tooltip';
import { cn } from '@/ui/cn';

import { commentsCopy } from '../copy';
import { useDeleteComment, useReply, useSetResolved } from '../queries';
import type { Comment, CommentThread } from '../types';
import { CommentBox } from './comment-box';

/** Props of {@link ThreadCard}. */
export interface ThreadCardProps {
  thread: CommentThread;
  /** Its span can no longer be found. */
  detached: boolean;
  /** Focus the reply field (a thread opened from its highlight). */
  autoFocus?: boolean;
}

/** The quoted span. */
function Quote({ thread, detached }: { thread: CommentThread; detached: boolean }) {
  return (
    <div className="flex items-start gap-2">
      <blockquote
        className={cn(
          'line-clamp-2 flex-1 border-l-2 pl-2 text-xs text-fg-muted',
          detached ? 'border-border line-through decoration-fg-subtle' : 'border-warning',
        )}
      >
        {thread.anchor.quote}
      </blockquote>
      {detached ? (
        <Hint label={commentsCopy.detachedHint} side="top">
          <span className="shrink-0 rounded-sm bg-bg-muted px-1.5 py-0.5 text-xs text-fg-muted">
            {commentsCopy.detached}
          </span>
        </Hint>
      ) : null}
    </div>
  );
}

/** One comment, with Delete on hover. */
function CommentItem({ comment }: { comment: Comment }) {
  const remove = useDeleteComment();
  return (
    <li className="group/comment flex items-start gap-2">
      <div className="min-w-0 flex-1">
        <p className="text-sm whitespace-pre-wrap text-fg">{comment.body}</p>
        <time dateTime={comment.createdAt} className="text-xs text-fg-muted">
          {formatDistanceToNowStrict(new Date(comment.createdAt), { addSuffix: true })}
        </time>
      </div>
      <IconButton
        icon={Trash2}
        label={commentsCopy.delete}
        className="opacity-0 group-hover/comment:opacity-100 focus-visible:opacity-100"
        onClick={() => {
          remove.mutate(comment.id);
        }}
      />
    </li>
  );
}

/** The card. */
export function ThreadCard({ thread, detached, autoFocus = false }: ThreadCardProps) {
  const reply = useReply();
  const resolve = useSetResolved();
  const resolved = thread.resolvedAt !== null;
  return (
    <article className={cn('flex flex-col gap-3', resolved && 'opacity-70')}>
      <div className="flex items-start gap-2">
        <div className="min-w-0 flex-1">
          <Quote thread={thread} detached={detached} />
        </div>
        <IconButton
          icon={resolved ? RotateCcw : CircleCheck}
          label={resolved ? commentsCopy.reopen : commentsCopy.resolve}
          onClick={() => {
            resolve.mutate({ threadId: thread.id, resolved: !resolved });
          }}
        />
      </div>
      <ul className="flex flex-col gap-2.5">
        {thread.comments.map((comment) => (
          <CommentItem key={comment.id} comment={comment} />
        ))}
      </ul>
      {resolved ? (
        <p className="flex items-center gap-1.5 text-xs text-fg-muted">
          <CircleCheck {...ICON_PROPS} size={12} />
          {commentsCopy.resolved}
        </p>
      ) : (
        <CommentBox
          autoFocus={autoFocus}
          placeholder={commentsCopy.replyPlaceholder}
          submitLabel={commentsCopy.reply}
          busy={reply.isPending}
          onSubmit={(body) => reply.mutateAsync({ threadId: thread.id, body })}
        />
      )}
    </article>
  );
}
