import { describe, expect, test } from 'vitest';

import { classifyAudioDrop } from './audio-drop';

describe('classifyAudioDrop', () => {
  test('imports the first audio file and skips other files', () => {
    expect(classifyAudioDrop(['/tmp/notes.txt', '/tmp/Call.M4A', '/tmp/b.wav'])).toEqual({
      kind: 'audio',
      path: '/tmp/Call.M4A',
    });
  });

  test('imports video files too', () => {
    expect(classifyAudioDrop(['/tmp/Screen Recording.mov'])).toEqual({
      kind: 'audio',
      path: '/tmp/Screen Recording.mov',
    });
    expect(classifyAudioDrop(['/tmp/call.webm'])).toEqual({
      kind: 'audio',
      path: '/tmp/call.webm',
    });
  });

  test('reports drops it cannot import so the user learns the formats', () => {
    expect(classifyAudioDrop(['/tmp/notes.txt', '/tmp/no-extension', '/tmp/a.wma'])).toEqual({
      kind: 'unsupported',
    });
  });

  test('leaves image drops to the notepad', () => {
    expect(classifyAudioDrop(['/tmp/board.PNG', '/tmp/b.jpeg'])).toEqual({ kind: 'nothing' });
  });

  test('does nothing for an empty drop', () => {
    expect(classifyAudioDrop([])).toEqual({ kind: 'nothing' });
  });
});
