import { describe, expect, test } from 'vitest';

import { formatList, isImportable } from './formats';

describe('import formats', () => {
  test('audio and video files import, in any case', () => {
    expect(isImportable('/a/Call.M4A')).toBe(true);
    expect(isImportable('/a/screen.mov')).toBe(true);
    expect(isImportable('/a/memo.opus')).toBe(true);
  });

  test('other files, dot files and names without an extension do not', () => {
    expect(isImportable('/a/notes.txt')).toBe(false);
    expect(isImportable('/a/.mp3')).toBe(false);
    expect(isImportable('/a.mp3/README')).toBe(false);
  });

  test('the list reads as a sentence', () => {
    expect(formatList()).toMatch(/^M4A, MP3, .* and WebM$/);
  });
});
