/**
 * Enhanced notes for `pnpm dev:mock`, written from a fixture without an
 * LLM: the user's own lines in ink (each with one grey detail when a fact
 * of the ground truth speaks to it), the decisions and action items in
 * grey, every grey line citing the transcript segment its words match best.
 * The shapes are what `deino_core::enhance::to_doc` writes, so the view
 * renders exactly as with real notes.
 */
import type {
  Block,
  Inline,
  listItem as ListItem,
  doc as NoteDoc,
  SegmentId,
} from '@/lib/ipc/generated/enhance';

import { fixtureUuid } from './fixture-ids';
import type { FixtureMeeting } from './fixture-schema';
import { markdownToDoc } from './mock-markdown';

type Author = 'user' | 'ai';

/** Lowercase words of three letters or more. */
function words(text: string): Set<string> {
  return new Set(text.toLowerCase().match(/[a-z0-9]{3,}/g) ?? []);
}

/** Shared words over all words (Jaccard). */
function overlap(a: Set<string>, b: Set<string>): number {
  const shared = [...a].filter((word) => b.has(word)).length;
  const union = new Set([...a, ...b]).size;
  return union === 0 ? 0 : shared / union;
}

/** The segment whose words match `text` best, as a citation. */
function cite(fixture: FixtureMeeting, text: string): Inline[] {
  const target = words(text);
  let best = -1;
  let score = 0;
  fixture.segments.forEach((segment, index) => {
    const value = overlap(target, words(segment.text));
    if (value > score) [best, score] = [index, value];
  });
  const segment = fixture.segments[best];
  if (segment === undefined) return [];
  const segmentId = fixtureUuid(`segment/${fixture.id}/${String(best)}`) as SegmentId;
  return [
    run(' ', 'ai'),
    { type: 'citation', attrs: { segmentId, startMs: segment.startMs }, marks: [origin('ai')] },
  ];
}

function origin(author: Author) {
  return { type: 'origin' as const, attrs: { author, edited: false } };
}

function run(text: string, author: Author): Inline {
  return { type: 'text', text, marks: [origin(author)] };
}

function paragraph(content: Inline[]): Block {
  return { type: 'paragraph', content };
}

function heading(text: string, author: Author): Block {
  return { type: 'heading', attrs: { level: 3 }, content: [run(text, author)] };
}

/** An AI bullet citing its best segment. */
function aiItem(fixture: FixtureMeeting, text: string): ListItem {
  return { type: 'listItem', content: [paragraph([run(text, 'ai'), ...cite(fixture, text)])] };
}

/** The plain text of inline content. */
function plain(content: readonly Inline[] | undefined): string {
  return (content ?? []).map((inline) => (inline.type === 'text' ? inline.text : '')).join('');
}

/** A user line, with the fact that answers it as a grey detail. */
function userItem(fixture: FixtureMeeting, line: Inline[]): ListItem {
  const text = plain(line);
  const user = line.map((inline): Inline =>
    inline.type === 'text'
      ? { ...inline, marks: [origin('user'), ...(inline.marks ?? [])] }
      : inline,
  );
  const fact = fixture.groundTruth.facts.find(
    (candidate) => overlap(words(text), words(candidate.answer)) > 0.12,
  );
  const content: Block[] = [paragraph(user)];
  if (fact !== undefined) {
    content.push({ type: 'bulletList', content: [aiItem(fixture, fact.answer)] });
  }
  return { type: 'listItem', content };
}

/** Each item's own line. */
function listLines(items: readonly { content: Block[] }[]): Inline[][] {
  return items.flatMap((item) => {
    const first = item.content[0];
    return first?.type === 'paragraph' ? [first.content ?? []] : [];
  });
}

/**
 * The user's lines, sections opened by their headings; lines before the
 * first heading go under "From your notes", like the engine's leftovers.
 */
function fromUserNotes(fixture: FixtureMeeting): Block[] {
  if (fixture.userNotes === null) return [];
  const source = markdownToDoc(fixture.userNotes).content;
  const blocks: Block[] = source[0]?.type === 'heading' ? [] : [heading('From your notes', 'ai')];
  let items: ListItem[] = [];
  const close = () => {
    if (items.length > 0) blocks.push({ type: 'bulletList', content: items });
    items = [];
  };
  for (const block of source) {
    if (block.type === 'heading') {
      close();
      blocks.push(heading(plain(block.content), 'user'));
    } else if (block.type === 'bulletList' || block.type === 'taskList') {
      items.push(...listLines(block.content).map((line) => userItem(fixture, line)));
    } else if (block.type === 'paragraph') {
      items.push(userItem(fixture, block.content ?? []));
    }
  }
  close();
  return blocks;
}

/** The fixture's enhanced notes. */
export function mockEnhancedDoc(fixture: FixtureMeeting): NoteDoc {
  const { decisions, actionItems } = fixture.groundTruth;
  const content: Block[] = [
    heading('Overview', 'ai'),
    { type: 'bulletList', content: decisions.slice(0, 2).map((text) => aiItem(fixture, text)) },
    ...fromUserNotes(fixture),
  ];
  if (decisions.length > 2) {
    content.push(heading('Decisions', 'ai'), {
      type: 'bulletList',
      content: decisions.slice(2).map((text) => aiItem(fixture, text)),
    });
  }
  if (actionItems.length > 0) {
    content.push(heading('Action items', 'ai'), {
      type: 'taskList',
      content: actionItems.map((item) => ({
        type: 'taskItem',
        attrs: { checked: false },
        content: [paragraph([run(item.title, 'ai'), ...cite(fixture, item.evidence)])],
      })),
    });
  }
  return { type: 'doc', content };
}
