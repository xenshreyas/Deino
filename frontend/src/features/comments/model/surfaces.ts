/**
 * The two kinds of comment surface (ADR 0048). The transcript anchors a
 * comment in one line: the bubble carrying `data-segment-id`, with offsets
 * in that line's text (a selection across lines keeps its first line's
 * part). The notes anchor in the whole document's text.
 */
import type {
  CommentAnchor,
  CommentSurface,
  CommentTarget,
  CommentThread,
  SegmentId,
} from '../types';
import { locateQuote } from './anchor';
import { rangeOf, textOffset } from './dom-text';

/** The attribute a transcript bubble names its line with. */
export const SEGMENT_ATTRIBUTE = 'data-segment-id';

/** The bubble holding `node`, inside `container`. */
export function bubbleOf(container: HTMLElement, node: Node): HTMLElement | null {
  const element = node instanceof HTMLElement ? node : node.parentElement;
  const bubble = element?.closest<HTMLElement>(`[${SEGMENT_ATTRIBUTE}]`) ?? null;
  return bubble !== null && container.contains(bubble) ? bubble : null;
}

/** The anchor of `range` in the transcript under `container`. */
function transcriptAnchor(container: HTMLElement, range: Range): CommentAnchor | null {
  const bubble = bubbleOf(container, range.startContainer);
  const attribute = bubble?.getAttribute(SEGMENT_ATTRIBUTE);
  if (bubble === null || attribute == null) return null;
  // Validated boundary: the bubble writes its line's `SegmentId` there.
  const segmentId = attribute as SegmentId;
  const text = bubble.textContent;
  const start = textOffset(bubble, range.startContainer, range.startOffset) ?? 0;
  const sameLine = bubbleOf(container, range.endContainer) === bubble;
  const end = sameLine
    ? (textOffset(bubble, range.endContainer, range.endOffset) ?? start)
    : text.length;
  const quote = text.slice(start, end);
  if (quote.trim() === '') return null;
  return { target: 'transcript', segmentId, start, end, quote };
}

/** The transcript under `container`. */
export function transcriptSurface(container: HTMLElement): CommentSurface {
  return {
    anchorOf: (range) => transcriptAnchor(container, range),
    shows: (thread) => thread.anchor.target === 'transcript',
    rangeOf: (thread: CommentThread) => {
      const { segmentId } = thread.anchor;
      if (segmentId === null) return null;
      const bubble = container.querySelector<HTMLElement>(`[${SEGMENT_ATTRIBUTE}="${segmentId}"]`);
      if (bubble === null) return null;
      const span = locateQuote(bubble.textContent, thread.anchor);
      return span === null ? null : rangeOf(bubble, span);
    },
  };
}

/** The class TipTap gives the editable document element. */
const EDITOR_SELECTOR = '.ProseMirror';

/** A notes document under `container` (its TipTap editor), as `target`. */
export function notesSurface(container: HTMLElement, target: CommentTarget): CommentSurface {
  const root = container.querySelector<HTMLElement>(EDITOR_SELECTOR) ?? container;
  return textSurface(root, target);
}

/** A text document under `root`, as `target`. */
export function textSurface(root: HTMLElement, target: CommentTarget): CommentSurface {
  return {
    anchorOf: (range) => {
      const start = textOffset(root, range.startContainer, range.startOffset);
      const end = textOffset(root, range.endContainer, range.endOffset);
      if (start === null || end === null || end <= start) return null;
      const quote = root.textContent.slice(start, end);
      if (quote.trim() === '') return null;
      return { target, segmentId: null, start, end, quote };
    },
    shows: (thread) => thread.anchor.target === target,
    rangeOf: (thread) => {
      const span = locateQuote(root.textContent, thread.anchor);
      return span === null ? null : rangeOf(root, span);
    },
  };
}
