import { beforeEach, describe, expect, it } from 'vitest';

import type { CommentThread, CommentThreadId, MeetingId, SegmentId } from '../types';
import { textSurface, transcriptSurface } from './surfaces';

const FIRST = '0192a5f0-0000-7000-8000-000000000001' as SegmentId;
const SECOND = '0192a5f0-0000-7000-8000-000000000002' as SegmentId;

/** A transcript of two bubbles, the second with a search hit inside it. */
function transcript(): HTMLElement {
  const container = document.createElement('div');
  container.innerHTML = `
    <div data-segment-id="${FIRST}">We ship on Friday.</div>
    <div data-segment-id="${SECOND}">The <mark>pricing</mark> page needs a rewrite.</div>`;
  document.body.replaceChildren(container);
  return container;
}

function bubble(container: HTMLElement, id: SegmentId): HTMLElement {
  const found = container.querySelector<HTMLElement>(`[data-segment-id="${id}"]`);
  if (found === null) throw new Error('no bubble');
  return found;
}

/** A range over `text` inside `element`'s text. */
function select(element: HTMLElement, text: string): Range {
  const start = element.textContent.indexOf(text);
  const range = document.createRange();
  const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT);
  let seen = 0;
  for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) {
    const length = node.textContent?.length ?? 0;
    if (start >= seen && start <= seen + length) range.setStart(node, start - seen);
    if (start + text.length >= seen && start + text.length <= seen + length) {
      range.setEnd(node, start + text.length - seen);
    }
    seen += length;
  }
  return range;
}

function thread(anchor: CommentThread['anchor']): CommentThread {
  return {
    id: 'thread-1' as CommentThreadId,
    meetingId: 'meeting-1' as MeetingId,
    anchor,
    resolvedAt: null,
    createdAt: '2026-10-12T10:00:00Z',
    comments: [],
  };
}

describe('transcriptSurface', () => {
  let container: HTMLElement;
  beforeEach(() => {
    container = transcript();
  });

  it('anchors a selection by its line and offsets in the line, across marks', () => {
    const anchor = transcriptSurface(container).anchorOf(
      select(bubble(container, SECOND), 'pricing page'),
    );

    expect(anchor).toEqual({
      target: 'transcript',
      segmentId: SECOND,
      start: 4,
      end: 16,
      quote: 'pricing page',
    });
  });

  it('finds the span again after another line is edited and lines are re-rendered', () => {
    const surface = transcriptSurface(container);
    const anchor = surface.anchorOf(select(bubble(container, SECOND), 'pricing page'));
    if (anchor === null) throw new Error('no anchor');

    bubble(container, FIRST).textContent = 'We ship on Monday instead.';
    container.prepend(bubble(container, SECOND));

    expect(surface.rangeOf(thread(anchor))?.toString()).toBe('pricing page');
  });

  it('draws nothing for a detached thread', () => {
    const surface = transcriptSurface(container);
    const gone = thread({
      target: 'transcript',
      segmentId: SECOND,
      start: 4,
      end: 16,
      quote: 'plans page',
    });

    expect(surface.rangeOf(gone)).toBeNull();
    expect(surface.rangeOf({ ...gone, anchor: { ...gone.anchor, segmentId: null } })).toBeNull();
  });
});

describe('textSurface', () => {
  it('anchors in the whole document and follows the words after an edit above them', () => {
    const root = document.createElement('div');
    root.innerHTML = '<h2>Plan</h2><p>Ship the beta on Friday.</p>';
    document.body.replaceChildren(root);
    const surface = textSurface(root, 'notes');
    const anchor = surface.anchorOf(select(root, 'the beta'));
    if (anchor === null) throw new Error('no anchor');

    root.querySelector('h2')?.replaceChildren('The new plan');

    expect(anchor).toMatchObject({ target: 'notes', segmentId: null, quote: 'the beta' });
    expect(surface.rangeOf(thread(anchor))?.toString()).toBe('the beta');
  });
});
