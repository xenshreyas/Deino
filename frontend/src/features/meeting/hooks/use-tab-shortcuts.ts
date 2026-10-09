import { useMountEffect } from '@/hooks/useMountEffect';

import { MEETING_TABS } from '../slots';
import type { MeetingTabId } from '../types';

/** The tab ⌘`digit` selects, if any (⌘1 is the first tab). */
export function tabForDigit(key: string): MeetingTabId | undefined {
  const index = Number.parseInt(key, 10) - 1;
  return Number.isInteger(index) ? MEETING_TABS[index]?.id : undefined;
}

/** Binds ⌘1…⌘4 to the meeting tabs while the meeting page is mounted. */
export function useTabShortcuts(onSelect: (tab: MeetingTabId) => void): void {
  useMountEffect(() => {
    const listener = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || event.shiftKey || event.altKey) return;
      const tab = tabForDigit(event.key);
      if (tab === undefined) return;
      event.preventDefault();
      onSelect(tab);
    };
    window.addEventListener('keydown', listener);
    return () => {
      window.removeEventListener('keydown', listener);
    };
  });
}
