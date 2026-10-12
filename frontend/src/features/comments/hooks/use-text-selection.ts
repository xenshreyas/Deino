/**
 * The text selected inside an element, and where it is on screen: follows
 * the document's `selectionchange`, ignoring selections elsewhere.
 */
import { type RefObject, useState } from 'react';

import { useMountEffect } from '@/hooks/useMountEffect';

import { screenRect } from '../store';
import type { ScreenRect } from '../types';

/** A live selection inside the element. */
export interface TextSelection {
  /** The element the selection is in. */
  root: HTMLElement;
  range: Range;
  rect: ScreenRect;
}

/** The selection inside `container`, if it holds a non-empty one. */
export function selectionIn(container: HTMLElement | null): TextSelection | null {
  const selection = window.getSelection();
  if (
    container === null ||
    selection === null ||
    selection.isCollapsed ||
    selection.rangeCount === 0
  ) {
    return null;
  }
  const range = selection.getRangeAt(0);
  if (!container.contains(range.commonAncestorContainer) || range.toString().trim() === '')
    return null;
  return { root: container, range, rect: screenRect(range.getBoundingClientRect()) };
}

/** The selection inside the element `ref` points at. */
export function useTextSelection(ref: RefObject<HTMLElement | null>): TextSelection | null {
  const [selection, setSelection] = useState<TextSelection | null>(null);
  useMountEffect(() => {
    const update = () => {
      setSelection(selectionIn(ref.current));
    };
    document.addEventListener('selectionchange', update);
    return () => {
      document.removeEventListener('selectionchange', update);
    };
  });
  return selection;
}
