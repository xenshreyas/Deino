import { useMountEffect } from '@/hooks/useMountEffect';

import { usePageStore } from '../page-store';

/** True while the user types in a field, where page keys must not fire. */
function isTyping(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName);
}

/** Runs the page shortcut `event` names; true when it did. */
function runShortcut(event: KeyboardEvent): boolean {
  const page = usePageStore.getState();
  const plain = !event.metaKey && !event.ctrlKey && !event.altKey;
  if (plain && event.key.toLowerCase() === 'f') {
    page.setMenuOpen(true);
    return true;
  }
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'a') {
    page.selectAll();
    return true;
  }
  const selecting = page.allSelected || page.selection.ids.size > 0;
  if (event.key === 'Escape' && selecting) {
    page.clearSelection();
    return true;
  }
  return false;
}

/**
 * The Action Items page's keys: `F` opens the filter menu, ⌘A selects every
 * listed item and Esc clears the selection. Typing in a field is left alone.
 * The page's state starts fresh each time the page opens.
 */
export function usePageShortcuts(): void {
  useMountEffect(() => {
    usePageStore.getState().reset();
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
