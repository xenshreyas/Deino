import { Editor, type JSONContent } from '@tiptap/core';
import { afterEach, beforeAll, describe, expect, it } from 'vitest';

import { loadNoteExtensions, toNoteDoc } from '@/features/notepad';

import { EditedAiText } from './edited-ai-text';

type Factory = Awaited<ReturnType<typeof loadNoteExtensions>>;

let noteExtensions: Factory;
let editor: Editor | null = null;

beforeAll(async () => {
  noteExtensions = await loadNoteExtensions();
});

afterEach(() => {
  editor?.destroy();
  editor = null;
});

/** A text run with an origin mark. */
function run(text: string, author: 'ai' | 'user', edited = false): JSONContent {
  return { type: 'text', text, marks: [{ type: 'origin', attrs: { author, edited } }] };
}

/** An editor over one paragraph of `runs`. */
function makeEditor(runs: JSONContent[]): Editor {
  editor = new Editor({
    extensions: [
      ...noteExtensions({
        placeholder: '',
        resolveImageSrc: (src) => src,
        mentions: null,
        searchPeople: () => Promise.resolve([]),
        templates: { menu: null, deps: null },
      }),
      EditedAiText,
    ],
    content: { type: 'doc', content: [{ type: 'paragraph', content: runs }] },
  });
  return editor;
}

/** Each text run's text and its origin attributes. */
function runs(view: Editor): { text: string; origin: unknown }[] {
  const [paragraph] = toNoteDoc(view.getJSON()).content;
  if (paragraph?.type !== 'paragraph') return [];
  return (paragraph.content ?? []).map((node) => ({
    text: node.type === 'text' ? node.text : '',
    origin: node.marks?.find((mark) => mark.type === 'origin')?.attrs,
  }));
}

describe('EditedAiText', () => {
  it('marks an AI run the user typed into as edited', () => {
    const view = makeEditor([run('Ship on Friday.', 'ai')]);

    view.commands.insertContentAt(6, 'it ');

    expect(runs(view)).toEqual([
      { text: 'Ship it on Friday.', origin: { author: 'ai', edited: true } },
    ]);
  });

  it('marks an AI run the user deleted from as edited', () => {
    const view = makeEditor([run('Ship on Friday.', 'ai')]);

    view.commands.deleteRange({ from: 6, to: 9 });

    expect(runs(view)).toEqual([{ text: 'Ship Friday.', origin: { author: 'ai', edited: true } }]);
  });

  it("leaves the user's own text and untouched AI text alone", () => {
    const view = makeEditor([run('my line ', 'user'), run('and the AI line.', 'ai')]);

    view.commands.insertContentAt(4, 'own ');

    expect(runs(view)).toEqual([
      { text: 'my own line ', origin: { author: 'user', edited: false } },
      { text: 'and the AI line.', origin: { author: 'ai', edited: false } },
    ]);
  });

  it('does nothing when the document did not change', () => {
    const view = makeEditor([run('Ship on Friday.', 'ai')]);

    view.commands.setTextSelection(3);

    expect(runs(view)).toEqual([
      { text: 'Ship on Friday.', origin: { author: 'ai', edited: false } },
    ]);
  });
});
