import { Editor } from '@tiptap/core';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { asTemplateId, type Template } from '@/features/templates';

import { toNoteDoc } from '../doc-schema';
import type { Person } from '../types';
import { noteExtensions } from './extensions';
import { MENTION_LIMIT, rankMentionCandidates, withNewPerson } from './mention-ranking';
import { createMenuStore, createMentionStore } from './menu-store';
import type { TemplateCommandOptions } from './template-command';
import { pastePlainText } from './plain-paste';

const PRIYA = '0192f3a0-6c1e-7b2a-9d4e-1f2a3b4c5d6e';

let editor: Editor | null = null;

const NO_TEMPLATES: TemplateCommandOptions = { menu: null, deps: null };

const STANDUP: Template = {
  id: asTemplateId('standup'),
  isBuiltin: true,
  content: {
    name: 'Standup',
    description: 'Daily standup',
    meetingContext: '',
    sections: [{ title: 'Blockers', instructions: '', format: 'bullets' }],
    autoRules: [],
  },
  createdAt: '2026-10-01T09:00:00Z',
  updatedAt: '2026-10-01T09:00:00Z',
};

function makeEditor(
  content: string | object = '',
  templates: TemplateCommandOptions = NO_TEMPLATES,
) {
  editor = new Editor({
    extensions: noteExtensions({
      placeholder: 'Write anything.',
      resolveImageSrc: (src) => `asset://${src}`,
      mentions: createMentionStore(),
      searchPeople: () => Promise.resolve([]),
      templates,
    }),
    content,
  });
  return editor;
}

/** Types `text` the way a keyboard does, so input rules run. */
function typeText(view: Editor, text: string): void {
  for (const character of text) {
    const { from, to } = view.state.selection;
    const handled = view.view.someProp('handleTextInput', (handle) =>
      handle(view.view, from, to, character, () => view.state.tr.insertText(character, from, to)),
    );
    if (handled !== true) view.view.dispatch(view.state.tr.insertText(character, from, to));
  }
}

afterEach(() => {
  editor?.destroy();
  editor = null;
});

function person(name: string, email: string | null, mergedInto: string | null = null): Person {
  return {
    id: crypto.randomUUID(),
    name,
    email,
    companyId: null,
    title: null,
    avatarPath: null,
    notesMd: '',
    summaryMd: null,
    summaryUpdatedAt: null,
    isMe: false,
    mergedInto,
    createdAt: '2026-10-08T09:00:00Z',
    updatedAt: '2026-10-08T09:00:00Z',
  } as Person;
}

describe('mention suggestions', () => {
  it('put name prefixes before word prefixes before emails, and drop merged people', () => {
    const people = [
      person('Ada Pri', null),
      person('Maya', 'pri.maya@example.com'),
      person('Priya Shah', 'priya@example.com'),
      person('Priya Old', null, PRIYA),
    ];

    const ranked = rankMentionCandidates(people, ' PRI ');

    expect(ranked.map((candidate) => candidate.label)).toEqual(['Priya Shah', 'Ada Pri', 'Maya']);
    expect(ranked[0]).toMatchObject({ detail: 'priya@example.com' });
  });

  it('never offer more than the menu shows', () => {
    const people = Array.from({ length: 12 }, (_, n) => person(`Person ${String(n)}`, null));
    expect(rankMentionCandidates(people, '')).toHaveLength(MENTION_LIMIT);
  });

  it('offer to add someone new only when nobody has the typed name', () => {
    const known = [{ id: 'a', label: 'Owen Price', detail: null }];

    expect(withNewPerson(known, ' owen  price ')).toEqual(known);
    expect(withNewPerson(known, 'a')).toEqual(known);
    expect(withNewPerson(known, 'Dana  Ortiz').at(-1)).toMatchObject({
      label: 'Dana Ortiz',
      isNew: true,
    });
  });

  it('move the highlight with the arrows and choose with return', () => {
    const store = createMentionStore();
    const chosen: string[] = [];
    store.getState().show({
      items: [
        { id: 'a', label: 'Ada', detail: null },
        { id: 'b', label: 'Bo', detail: null },
      ],
      rect: null,
      choose: (candidate) => chosen.push(candidate.label),
    });

    expect(store.getState().key('ArrowUp')).toBe(true);
    expect(store.getState().active).toBe(1);
    expect(store.getState().key('Enter')).toBe(true);
    expect(chosen).toEqual(['Bo']);
    expect(store.getState().key('Escape')).toBe(true);
    expect(store.getState().session).toBeNull();
    expect(store.getState().key('Enter')).toBe(false);
  });
});

describe('the note editor', () => {
  it('pastes plain text as text, never as markup', () => {
    const view = makeEditor('<p>start</p>');
    view.commands.focus('end');

    pastePlainText(view, ' <b>bold?</b>\nsecond line');

    expect(view.getText()).toContain('start <b>bold?</b>');
    expect(view.getHTML()).not.toContain('<strong>');
    expect(view.getHTML()).not.toContain('<b>');
  });

  it.each([
    ['# ', 'heading'],
    ['- ', 'bulletList'],
    ['* ', 'bulletList'],
    ['1. ', 'orderedList'],
    ['[] ', 'taskList'],
    ['[x] ', 'taskList'],
  ])('turns %j typed at a line start into a %s', (typed, type) => {
    const view = makeEditor();
    view.commands.focus();
    typeText(view, typed);
    expect(view.getJSON().content[0]?.type).toBe(type);
  });

  it('turns **double stars** into bold', () => {
    const view = makeEditor();
    view.commands.focus();
    typeText(view, 'a **bold** ');
    expect(view.getHTML()).toContain('<strong>bold</strong>');
  });

  it('produces JSON the notes command accepts, mentions and citations included', () => {
    const view = makeEditor({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            { type: 'text', text: 'ask ' },
            { type: 'mention', attrs: { id: PRIYA, label: 'Priya' } },
            {
              type: 'text',
              text: ' retention fell',
              marks: [{ type: 'origin', attrs: { author: 'ai', edited: false } }],
            },
            {
              type: 'citation',
              attrs: { segmentId: PRIYA, startMs: 64_000 },
            },
          ],
        },
        { type: 'image', attrs: { src: `images/${PRIYA}.png` } },
      ],
    });

    const doc = toNoteDoc(view.getJSON());

    expect(doc.content[0]).toMatchObject({
      type: 'paragraph',
      content: [
        { type: 'text', text: 'ask ', marks: [] },
        { type: 'mention', attrs: { id: PRIYA, label: 'Priya' }, marks: [] },
        { type: 'text', marks: [{ type: 'origin', attrs: { author: 'ai', edited: false } }] },
        { type: 'citation', attrs: { segmentId: PRIYA, startMs: 64_000 } },
      ],
    });
    expect(doc.content[1]).toEqual({
      type: 'image',
      attrs: { src: `images/${PRIYA}.png`, alt: null, title: null },
    });
    expect(view.getHTML()).toContain(`src="asset://images/${PRIYA}.png"`);
    expect(view.getHTML()).toContain('1:04');
  });

  it('refuses links to anything but the web and mail', () => {
    const view = makeEditor(
      '<p><a href="javascript:alert(1)">x</a> <a href="https://a.io">y</a></p>',
    );
    expect(view.getHTML()).not.toContain('javascript:');
    expect(view.getHTML()).toContain('https://a.io');
  });

  it('runs /template: sets the meeting template and scaffolds its sections', async () => {
    const menu = createMenuStore<Template>();
    const setMeetingTemplate = vi.fn();
    const view = makeEditor('', { menu, deps: { templates: () => [STANDUP], setMeetingTemplate } });
    view.commands.focus();

    typeText(view, '/templ');
    await vi.waitFor(() => {
      expect(menu.getState().session?.items).toEqual([STANDUP]);
    });
    menu.getState().key('Enter');

    expect(setMeetingTemplate).toHaveBeenCalledWith('standup');
    expect(view.getHTML()).toContain('<h2>Blockers</h2>');
    expect(view.getText()).not.toContain('/templ');
  });
});
