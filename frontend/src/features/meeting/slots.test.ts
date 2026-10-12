import { describe, expect, test, vi } from 'vitest';

import { tabForDigit } from './hooks/use-tab-shortcuts';
import { meetingTabSchema } from './search';
import { MEETING_PANELS, MEETING_TABS } from './slots';

// The Notes and Transcript slots reach the legacy session; keep it out of this test.
vi.mock('./slots/transcript-slot', () => ({ TranscriptSlot: () => null }));

describe('meeting slots', () => {
  test('the tabs registry and the URL ?tab= schema list the same tabs, in order', () => {
    expect(MEETING_TABS.map((tab) => tab.id)).toEqual(meetingTabSchema.options);
  });

  test('two tabs and two panel sections (UX-03, UX-04)', () => {
    expect(MEETING_TABS.map((tab) => tab.label)).toEqual(['Notes', 'Transcript']);
    expect(MEETING_PANELS.map((panel) => panel.label)).toEqual(['Chat', 'Details']);
  });

  test('⌘1 and ⌘2 select the tabs in their registry order', () => {
    expect(tabForDigit('1')).toBe('notes');
    expect(tabForDigit('2')).toBe('transcript');
    expect(tabForDigit('3')).toBeUndefined();
    expect(tabForDigit('k')).toBeUndefined();
    expect(MEETING_TABS.map((tab) => tab.shortcut)).toEqual(['⌘1', '⌘2']);
  });
});
