/**
 * The notes while they are written (streamed notes): each finished line
 * fades in where the saved notes will have it, the page never scrolls on
 * its own, and "Writing below" offers a jump when the newest line is out
 * of view. Editing waits for the saved version; a click says so. A draft
 * from an answer being replaced (a retry) stays dimmed until the new
 * answer's first line arrives.
 */
import { type ReactNode, useRef, useState } from 'react';

import { useMountEffect } from '@/hooks/useMountEffect';
import { cn } from '@/ui/cn';
import { ArrowDown, ICON_PROPS } from '@/ui/icons';

import { notesCopy } from '../copy';
import type { NotesDraft } from '../types';
import { DocView } from './doc-view';

/** How far below the visible area the newest line must be to offer the jump, in px. */
const BELOW_MARGIN = 24;

/** Props of {@link DraftNotes}. */
export interface DraftNotesProps {
  draft: NotesDraft;
  /** What follows the notes inside their scroll. */
  footer?: ReactNode;
  /** The user clicked the notes, which cannot be edited yet. */
  onLocked: () => void;
}

/** True while the end of `content` is below what `scroller` shows. */
function useWritingBelow() {
  const scroller = useRef<HTMLDivElement>(null);
  const content = useRef<HTMLDivElement>(null);
  const [below, setBelow] = useState(false);
  const measure = () => {
    const outer = scroller.current?.getBoundingClientRect();
    const inner = content.current?.getBoundingClientRect();
    if (outer === undefined || inner === undefined) return;
    setBelow(inner.bottom > outer.bottom + BELOW_MARGIN);
  };
  useMountEffect(() => {
    if (typeof ResizeObserver === 'undefined' || content.current === null) return undefined;
    const observer = new ResizeObserver(measure);
    observer.observe(content.current);
    return () => {
      observer.disconnect();
    };
  });
  const jump = () => {
    content.current?.lastElementChild?.scrollIntoView({ behavior: 'smooth', block: 'center' });
  };
  return { scroller, content, below, measure, jump };
}

/** The draft, read-only. */
export function DraftNotes({ draft, footer, onLocked }: DraftNotesProps) {
  const { scroller, content, below, measure, jump } = useWritingBelow();
  return (
    <div className="relative flex min-h-0 flex-1 flex-col">
      <div
        ref={scroller}
        onScroll={measure}
        className="-ml-9 min-h-0 flex-1 overflow-y-auto pb-[30vh] pl-9"
      >
        <div
          ref={content}
          aria-label={notesCopy.editorLabel}
          aria-busy="true"
          aria-readonly="true"
          onPointerDown={onLocked}
          className={cn(
            'note-content enhanced-notes notes-draft cursor-default transition-opacity duration-(--dur-base)',
            draft.stale && 'opacity-50',
          )}
        >
          <DocView doc={draft.doc} />
        </div>
        {footer}
      </div>
      {below ? <WritingBelow onJump={jump} /> : null}
    </div>
  );
}

function WritingBelow({ onJump }: { onJump: () => void }) {
  return (
    <button
      type="button"
      onClick={onJump}
      className="absolute bottom-5 left-1/2 flex h-7 -translate-x-1/2 items-center gap-1.5 rounded-full bg-surface-raised px-3 text-xs text-fg shadow-pill"
    >
      <ArrowDown {...ICON_PROPS} size={14} />
      {notesCopy.writingBelow}
    </button>
  );
}
