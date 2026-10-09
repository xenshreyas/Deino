import { describe, expect, it } from 'vitest';

import { formatBytes, formatTime } from './format';

describe('formatBytes', () => {
  it('shows small sizes in bytes and larger ones in binary units', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(2048)).toBe('2.0 KB');
    expect(formatBytes(48_213)).toBe('47 KB');
    expect(formatBytes(731_357_568)).toBe('697 MB');
    expect(formatBytes(1.5 * 1024 ** 3)).toBe('1.5 GB');
  });
});

describe('formatTime', () => {
  it('shows the day and a 24-hour time to the second', () => {
    const shown = formatTime('2026-10-09T09:30:05Z');

    expect(shown).toMatch(/^Oct \d{1,2}, \d{2}:\d{2}:05$/);
  });
});
