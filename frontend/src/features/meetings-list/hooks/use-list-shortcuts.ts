import { useMountEffect } from '@/hooks/useMountEffect';

import { selectAllMatching, trashSelection } from '../actions';
import { usePageStore } from '../page-store';
import type { MeetingFilter } from '../types';

/** True while the user types in a field, where page keys must not fire. */
function isTyping(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName);
}

/** Runs the page shortcut `event` names; true when it did. */
function runShortcut(event: KeyboardEvent): boolean {
  const page = usePageStore.getState();
  const command = event.metaKey || event.ctrlKey;
  if (!command && !event.altKey && event.key.toLowerCase() === 'f') {
    page.setMenuOpen(true);
    return true;
  }
  if (command && event.key.toLowerCase() === 'a') {
    void selectAllMatching();
    return true;
  }
  const selecting = page.selection.ids.size > 0;
  if (event.key === 'Escape' && selecting) {
    page.clearSelection();
    return true;
  }
  if (command && event.key === 'Backspace' && selecting) {
    trashSelection();
    return true;
  }
  return false;
}

/**
 * The Meetings page's keys (Circleback §14): `F` opens the filter menu, ⌘A
 * selects every matching meeting, Esc clears the selection and ⌘⌫ moves the
 * selection to the trash. Typing in a field is left alone. The page's state
 * starts fresh, showing `filter`, each time the page opens.
 */
export function useListShortcuts(filter: MeetingFilter): void {
  useMountEffect(() => {
    usePageStore.getState().reset(filter);
    const listener = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.isComposing || isTyping(event.target)) return;
      if (runShortcut(event)) event.preventDefault();
    };
    window.addEventListener('keydown', listener);
    return () => {
      window.removeEventListener('keydown', listener);
    };
  });
}
