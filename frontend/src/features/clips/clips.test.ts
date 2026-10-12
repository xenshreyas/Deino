import { describe, expect, it } from 'vitest';

import { suggestName } from './components/clip-dialog';
import { momentLink } from './moment-link';
import type { MeetingId } from './types';

describe('momentLink', () => {
  it('links to the meeting at a whole millisecond', () => {
    expect(momentLink('meeting-fixture-standup' as MeetingId, 231_259.7)).toBe(
      'deino://meeting/meeting-fixture-standup?t=231259',
    );
  });

  it('escapes the meeting ID and never goes below zero', () => {
    expect(momentLink('a b/c' as MeetingId, -5)).toBe('deino://meeting/a%20b%2Fc?t=0');
  });
});

describe('suggestName', () => {
  it('collapses whitespace and shortens long selections', () => {
    expect(suggestName('  The   pricing\npage ')).toBe('The pricing page');
    const long = suggestName('word '.repeat(30));
    expect(long.endsWith('…')).toBe(true);
    expect(long.length).toBeLessThanOrEqual(61);
  });
});
