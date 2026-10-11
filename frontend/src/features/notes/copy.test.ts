import { describe, expect, it } from 'vitest';

import type { NoteDoc } from '@/features/notepad';

import { tocEntries } from './components/notes-toc';
import { clockLabel, reasonsText } from './copy';

describe('reasonsText', () => {
  it('reads one reason as it is', () => {
    expect(reasonsText(['attendeesChanged'])).toBe('attendees changed');
  });

  it('joins several with commas and a final "and"', () => {
    expect(reasonsText(['notesEdited', 'transcriptEdited', 'audioAppended'])).toBe(
      'you edited your notes, the transcript was edited and more of the meeting was transcribed',
    );
  });

  it('is empty without reasons', () => {
    expect(reasonsText([])).toBe('');
  });
});

describe('clockLabel', () => {
  it('writes minutes and seconds under an hour', () => {
    expect(clockLabel(0)).toBe('0:00');
    expect(clockLabel(271_900)).toBe('4:31');
  });

  it('adds hours from an hour on, as deino-doc writes citations', () => {
    expect(clockLabel(3_871_000)).toBe('1:04:31');
  });
});

describe('tocEntries', () => {
  it('lists the non-empty headings in order', () => {
    const doc: NoteDoc = {
      type: 'doc',
      content: [
        { type: 'heading', attrs: { level: 3 }, content: [{ type: 'text', text: 'Overview' }] },
        { type: 'paragraph', content: [{ type: 'text', text: 'Body' }] },
        { type: 'heading', attrs: { level: 3 }, content: [] },
        { type: 'heading', attrs: { level: 2 }, content: [{ type: 'text', text: 'Decisions ' }] },
      ],
    };

    expect(tocEntries(doc)).toEqual([
      { index: 0, text: 'Overview', level: 3 },
      { index: 1, text: 'Decisions', level: 2 },
    ]);
  });
});
