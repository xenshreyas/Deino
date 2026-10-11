/**
 * The frame around the rendered notes that shows the magnifier (NOT-03):
 * beside the cited line under the pointer, or, for the keyboard, beside the
 * line the caret is in; ⌘⇧S then opens its sources. While the popover is
 * open the magnifier stays put, whatever the pointer does.
 */
import type { Editor } from '@tiptap/react';
import { type MouseEvent, type ReactNode, type RefObject, useRef, useState } from 'react';

import type { MeetingId } from '@/features/meetings';
import { useMountEffect } from '@/hooks/useMountEffect';

import type { CitationRef } from '../types';
import { citedBlockAt, offsetWithin } from './cited-block';
import { CitationPopover } from './citation-popover';

/** The magnifier's line, and where it sits. */
interface Active {
  citations: CitationRef[];
  top: number;
}

/** The line's text is 26px tall; the 24px magnifier centres on it. */
const LINE_OFFSET = 1;

/** Props of {@link CitedFrame}. */
export interface CitedFrameProps {
  meetingId: MeetingId;
  editor: Editor;
  children: ReactNode;
}

/** ⌘⇧S opens the sources of the caret's line. */
function isSourcesShortcut(event: KeyboardEvent): boolean {
  const mod = event.metaKey || event.ctrlKey;
  return mod && event.shiftKey && !event.altKey && event.key.toLowerCase() === 's';
}

/**
 * Whether `node` is between lines: the gutter, a list's indent, the gap
 * between blocks or the magnifier itself. Moving there, on the way to the
 * magnifier, keeps it where it is.
 */
function inGutter(node: Node | null, root: HTMLElement): boolean {
  const element = node instanceof Element ? node : (node?.parentElement ?? null);
  if (element === null || element.closest('[data-sources-trigger]') !== null) return true;
  const block = element.closest('li, p');
  return block === null || !root.contains(block);
}

/** Where the magnifier is and whether its popover is open. */
function useMagnifier(frame: RefObject<HTMLDivElement | null>) {
  // Read by listeners registered once, so a ref, written in event handlers.
  const isOpen = useRef(false);
  const [active, setActive] = useState<Active | null>(null);
  const [open, setOpenState] = useState(false);
  const setOpen = (next: boolean) => {
    isOpen.current = next;
    setOpenState(next);
  };
  /** Beside the cited line `node` is in; none when it has no sources. */
  const place = (node: Node | null) => {
    const root = frame.current;
    if (root === null || isOpen.current) return;
    const block = citedBlockAt(node, root);
    const top = block === null ? 0 : offsetWithin(block.element, root) + LINE_OFFSET;
    setActive(block === null ? null : { citations: block.citations, top });
  };
  /** Follows the pointer, staying put while it crosses the gutter. */
  const follow = (node: Node | null) => {
    const root = frame.current;
    if (root !== null && !inGutter(node, root)) place(node);
  };
  const hide = () => {
    if (!isOpen.current) setActive(null);
  };
  return { active, open, setOpen, place, follow, hide };
}

/** The notes with their magnifier. */
export function CitedFrame({ meetingId, editor, children }: CitedFrameProps) {
  const frame = useRef<HTMLDivElement>(null);
  const { active, open, setOpen, place, follow, hide } = useMagnifier(frame);
  useCaretSources(editor, place, () => {
    setOpen(true);
  });
  return (
    <div
      ref={frame}
      className="relative -ml-9 pl-9"
      onMouseMove={(event: MouseEvent) => {
        follow(event.target instanceof Node ? event.target : null);
      }}
      onMouseLeave={hide}
    >
      {children}
      {active === null ? null : (
        <CitationPopover
          meetingId={meetingId}
          citations={active.citations}
          top={active.top}
          open={open}
          onOpenChange={setOpen}
        />
      )}
    </div>
  );
}

/** Follows the caret into cited lines, and opens their sources on ⌘⇧S. */
function useCaretSources(editor: Editor, show: (node: Node | null) => void, open: () => void) {
  useMountEffect(() => {
    const follow = () => {
      const { from } = editor.state.selection;
      show(editor.view.domAtPos(from).node);
    };
    const keydown = (event: KeyboardEvent) => {
      if (!isSourcesShortcut(event) || !editor.isFocused) return;
      event.preventDefault();
      follow();
      open();
    };
    editor.on('selectionUpdate', follow);
    window.addEventListener('keydown', keydown);
    return () => {
      editor.off('selectionUpdate', follow);
      window.removeEventListener('keydown', keydown);
    };
  });
}
