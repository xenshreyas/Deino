import { describe, expect, test } from 'vitest';

import { classifyAudioDrop } from './audio-drop';

describe('classifyAudioDrop', () => {
  test('ignores every drop while the import beta is off', () => {
    expect(classifyAudioDrop(['/tmp/call.mp3'], false)).toEqual({ kind: 'import-disabled' });
  });

  test('imports the first audio file and skips other files', () => {
    expect(classifyAudioDrop(['/tmp/notes.txt', '/tmp/Call.M4A', '/tmp/b.wav'], true)).toEqual({
      kind: 'audio',
      path: '/tmp/Call.M4A',
    });
  });

  test('reports non-audio drops so the user learns the supported formats', () => {
    expect(classifyAudioDrop(['/tmp/notes.txt', '/tmp/no-extension'], true)).toEqual({
      kind: 'unsupported',
    });
  });

  test('does nothing for an empty drop', () => {
    expect(classifyAudioDrop([], true)).toEqual({ kind: 'nothing' });
  });
});
