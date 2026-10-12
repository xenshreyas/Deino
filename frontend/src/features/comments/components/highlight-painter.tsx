/**
 * Paints the open threads' spans on a surface with the CSS Custom
 * Highlight API (`::highlight(deino-comment-*)`, `styles/comments.css`):
 * no DOM changes, so the transcript's word marks, search hits and the
 * editor stay untouched. The transcript is virtualised and the notes are
 * edited, so it repaints on every change under the surface. Render it with
 * a key that changes with the threads.
 */
import type { RefObject } from 'react';

import { useMountEffect } from '@/hooks/useMountEffect';

import type { CommentSurface, CommentTarget, CommentThread } from '../types';

/** The highlight's name for a surface of `target`. */
export function highlightName(target: CommentTarget): string {
  return `deino-comment-${target}`;
}

/** Whether this webview has the Custom Highlight API (WebKit 17.2+, Chromium 105+). */
function canHighlight(): boolean {
  return typeof Highlight !== 'undefined' && 'highlights' in CSS;
}

/** The DOM ranges of the threads to paint now. */
export function paintedRanges(surface: CommentSurface, threads: readonly CommentThread[]): Range[] {
  return threads
    .filter((thread) => thread.resolvedAt === null && surface.shows(thread))
    .map((thread) => surface.rangeOf(thread))
    .filter((range): range is Range => range !== null);
}

/** Props of {@link HighlightPainter}. */
export interface HighlightPainterProps {
  container: RefObject<HTMLElement | null>;
  surface: (container: HTMLElement) => CommentSurface;
  target: CommentTarget;
  threads: readonly CommentThread[];
}

/** Renders nothing; keeps the highlight in step with the surface. */
export function HighlightPainter({ container, surface, target, threads }: HighlightPainterProps) {
  useMountEffect(() => {
    const root = container.current;
    if (root === null || !canHighlight()) return;
    const name = highlightName(target);
    let frame = 0;
    const paint = () => {
      CSS.highlights.set(name, new Highlight(...paintedRanges(surface(root), threads)));
    };
    const observer = new MutationObserver(() => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(paint);
    });
    observer.observe(root, { subtree: true, childList: true, characterData: true });
    paint();
    return () => {
      observer.disconnect();
      cancelAnimationFrame(frame);
      CSS.highlights.delete(name);
    };
  });
  return null;
}
