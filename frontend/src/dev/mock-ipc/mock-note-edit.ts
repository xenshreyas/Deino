/**
 * The edits `pnpm dev:mock` proposes, standing in for
 * `deino_core::notes::edit`: a plain rename done like Rust (whole words,
 * ignoring case, mention labels too, never a link's target or a citation),
 * and for anything else a canned "AI" bullet after the selection (or the
 * first list). The diff is a simpler version of Rust's.
 */
import type {
  Block,
  DiffBlock,
  DiffSegment,
  doc as NoteDoc,
  EditProposal,
  EditRequest,
  Inline,
  TextKind,
} from '@/lib/ipc/generated/notes-edit';

/** The canned AI addition. */
const ADDITION = 'Follow-up: confirm the owner and the date with the team.';

const RENAME_PATTERNS: readonly [RegExp, boolean][] = [
  [/^rename\s+(.+?)\s+(?:to|as)\s+(.+?)(?:\s+everywhere)?$/i, false],
  [/^(.+?)\s+is\s+(?:actually\s+)?(?:spelled|spelt)\s+(.+)$/i, false],
  [/^replace\s+(.+?)\s+with\s+(.+?)(?:\s+everywhere)?$/i, false],
  [/^it'?s\s+(.+?),?\s+not\s+(.+)$/i, true],
];

interface Textblock {
  kind: TextKind;
  content: Inline[];
  section: string | null;
}

/** Every paragraph, heading and code block, in reading order. */
function textblocks(doc: NoteDoc): Textblock[] {
  const found: Textblock[] = [];
  let section: string | null = null;
  const visit = (blocks: readonly Block[], lead: TextKind | null) => {
    blocks.forEach((block, index) => {
      const kind = index === 0 && lead !== null ? lead : 'paragraph';
      if (block.type === 'heading') section = plain(block.content ?? []);
      if (block.type === 'paragraph' || block.type === 'heading' || block.type === 'codeBlock') {
        const shown =
          block.type === 'heading' ? 'heading' : block.type === 'codeBlock' ? 'code' : kind;
        found.push({ kind: shown, content: block.content ?? [], section });
      } else if (block.type === 'bulletList' || block.type === 'orderedList') {
        block.content.forEach((item) => {
          visit(item.content, block.type === 'bulletList' ? 'bullet' : 'numbered');
        });
      } else if (block.type === 'taskList') {
        block.content.forEach((item) => {
          visit(item.content, 'todo');
        });
      } else if (block.type === 'blockquote') {
        visit(block.content, 'quote');
      }
    });
  };
  visit(doc.content, null);
  return found;
}

function plain(content: readonly Inline[]): string {
  return content
    .map((inline) =>
      inline.type === 'text'
        ? inline.text
        : inline.type === 'mention'
          ? `@${inline.attrs.label}`
          : '',
    )
    .join('')
    .trim();
}

/** `[from, to]` when the request is a plain rename. */
function parseRename(instruction: string): [string, string] | null {
  const request = instruction.trim().replace(/[.!]+$/, '');
  for (const [pattern, reversed] of RENAME_PATTERNS) {
    const match = pattern.exec(request);
    const [first, second] = [match?.[1]?.trim(), match?.[2]?.trim()];
    if (first === undefined || second === undefined || /^(the|a|an|this|that)\s/i.test(first))
      continue;
    return reversed ? [second, first] : [first, second];
  }
  return null;
}

/** The proposal of a rename, or `null` when the request is not one or matches nothing. */
export function mockRename(
  current: NoteDoc,
  request: EditRequest,
  base: number,
): EditProposal | null {
  const names = parseRename(request.instruction);
  if (names === null) return null;
  const escaped = names[0].replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const pattern = new RegExp(`(?<![\\p{L}\\p{N}_])${escaped}(?![\\p{L}\\p{N}_])`, 'giu');
  const doc = structuredClone(current);
  let count = 0;
  textblocks(doc).forEach((block, index) => {
    const selection = request.selection ?? [];
    if (selection.length > 0 && !selection.includes(index)) return;
    for (const inline of block.content) {
      const swap = (text: string) =>
        text.replace(pattern, () => {
          count += 1;
          return names[1];
        });
      if (inline.type === 'text') inline.text = swap(inline.text);
      if (inline.type === 'mention') inline.attrs.label = swap(inline.attrs.label);
    }
  });
  if (count === 0) return null;
  return proposal(current, doc, { base, method: 'rename', occurrences: count, request });
}

/** The canned AI edit: one AI bullet after the selection, or after the first list's first item. */
export function mockAddition(current: NoteDoc, request: EditRequest, base: number): EditProposal {
  const doc = structuredClone(current);
  const item = {
    type: 'listItem' as const,
    content: [{ type: 'paragraph' as const, content: [aiText(ADDITION)] }],
  };
  const list = doc.content.find((block) => block.type === 'bulletList');
  if (list?.type === 'bulletList') list.content.splice(1, 0, item);
  else doc.content.push({ type: 'bulletList', content: [item] });
  return proposal(current, doc, { base, method: 'ai', occurrences: null, request });
}

function aiText(text: string): Inline {
  return {
    type: 'text',
    text,
    marks: [{ type: 'origin', attrs: { author: 'ai', edited: false } }],
  };
}

function proposal(
  before: NoteDoc,
  after: NoteDoc,
  how: {
    base: number;
    method: EditProposal['method'];
    occurrences: number | null;
    request: EditRequest;
  },
): EditProposal {
  return {
    meetingId: how.request.meetingId,
    baseVersion: how.base,
    method: how.method,
    occurrences: how.occurrences,
    doc: after,
    changes: diff(textblocks(before), textblocks(after)),
  };
}

/** Changed blocks pair up when the count is the same; otherwise the new ones are added. */
function diff(old: Textblock[], next: Textblock[]): DiffBlock[] {
  if (old.length === next.length) {
    return next.flatMap((block, index) => {
      const before = plain(old[index]?.content ?? []);
      const after = plain(block.content);
      return before === after
        ? []
        : [
            {
              change: 'changed',
              kind: block.kind,
              section: block.section,
              segments: words(before, after),
            },
          ];
    });
  }
  const known = new Set(old.map((block) => plain(block.content)));
  return next
    .filter((block) => !known.has(plain(block.content)))
    .map((block) => ({
      change: 'added',
      kind: block.kind,
      section: block.section,
      segments: [{ change: 'added', text: plain(block.content) }],
    }));
}

/** A word diff over the common start and end. */
function words(before: string, after: string): DiffSegment[] {
  const a = before.split(/(\s+)/);
  const b = after.split(/(\s+)/);
  let start = 0;
  while (start < a.length && a[start] === b[start]) start += 1;
  let end = 0;
  while (end < a.length - start && end < b.length - start && a.at(-1 - end) === b.at(-1 - end))
    end += 1;
  const segments: DiffSegment[] = [
    { change: 'same', text: a.slice(0, start).join('') },
    { change: 'removed', text: a.slice(start, a.length - end).join('') },
    { change: 'added', text: b.slice(start, b.length - end).join('') },
    { change: 'same', text: a.slice(a.length - end).join('') },
  ];
  return segments.filter((segment) => segment.text !== '');
}
