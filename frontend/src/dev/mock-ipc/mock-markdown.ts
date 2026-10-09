/**
 * A small Markdown ↔ document converter for `pnpm dev:mock`, standing in for
 * `deino-doc` (Rust) behind the `notes` plugin mock. It covers what the
 * fixtures' notes and the generated summaries use: `#` headings, `-`/`*`
 * and `1.` lists nested by indentation, `[ ]`/`[x]` items, paragraphs (lines
 * kept as hard breaks), and `**bold**`, `*italic*` and `` `code` ``. The
 * real app always converts in Rust; this only has to look right.
 */
import type { Block, Inline, Mark, doc as NoteDoc } from '@/lib/ipc/generated/notes';

/** `**bold**`, `*italic*` and `` `code` `` runs inside a line. */
const INLINE = /(\*\*[^*]+\*\*|\*[^*\s][^*]*\*|`[^`]+`)/g;

function inlines(text: string): Inline[] {
  return text
    .split(INLINE)
    .filter((part) => part !== '')
    .map((part): Inline => {
      const mark = (type: Mark['type'], trim: number): Inline => ({
        type: 'text',
        text: part.slice(trim, -trim),
        marks: [{ type } as Mark],
      });
      if (part.startsWith('**')) return mark('bold', 2);
      if (part.startsWith('`')) return mark('code', 1);
      if (part.startsWith('*') && part.endsWith('*') && part.length > 2) return mark('italic', 1);
      return { type: 'text', text: part, marks: [] };
    });
}

interface Line {
  indent: number;
  kind: 'bullet' | 'number' | 'task' | 'heading' | 'text';
  text: string;
  level: number;
  checked: boolean;
}

function parseLine(raw: string): Line {
  const indent = raw.length - raw.trimStart().length;
  const text = raw.trim();
  const heading = /^(#{1,6})\s+(.*)$/.exec(text);
  if (heading)
    return {
      indent,
      kind: 'heading',
      text: heading[2] ?? '',
      level: heading[1]?.length ?? 1,
      checked: false,
    };
  const task = /^[-*]\s+\[([ xX])\]\s*(.*)$/.exec(text);
  if (task)
    return { indent, kind: 'task', text: task[2] ?? '', level: 0, checked: task[1] !== ' ' };
  const bullet = /^[-*+]\s+(.*)$/.exec(text);
  if (bullet) return { indent, kind: 'bullet', text: bullet[1] ?? '', level: 0, checked: false };
  const number = /^\d+[.)]\s+(.*)$/.exec(text);
  if (number) return { indent, kind: 'number', text: number[1] ?? '', level: 0, checked: false };
  return { indent, kind: 'text', text, level: 0, checked: false };
}

/** Builds a list (and its nested lists) starting at `lines[start]`. */
function list(lines: Line[], start: number): [Block, number] {
  const first = lines[start];
  const kind = first?.kind ?? 'bullet';
  const items: { checked: boolean; content: Block[] }[] = [];
  let index = start;
  while (index < lines.length) {
    const line = lines[index];
    if (line === undefined || line.kind !== kind || line.indent !== first?.indent) break;
    const content: Block[] = [{ type: 'paragraph', content: inlines(line.text) }];
    index += 1;
    const next = lines[index];
    if (
      next !== undefined &&
      next.indent > line.indent &&
      next.kind !== 'text' &&
      next.kind !== 'heading'
    ) {
      const [nested, after] = list(lines, index);
      content.push(nested);
      index = after;
    }
    items.push({ checked: line.checked, content });
  }
  return [listBlock(kind, items), index];
}

/** The list node for parsed items. */
function listBlock(kind: Line['kind'], items: { checked: boolean; content: Block[] }[]): Block {
  if (kind === 'task') {
    return {
      type: 'taskList',
      content: items.map((item) => ({
        type: 'taskItem',
        attrs: { checked: item.checked },
        content: item.content,
      })),
    };
  }
  const listItems = items.map((item) => ({ type: 'listItem' as const, content: item.content }));
  return kind === 'number'
    ? { type: 'orderedList', attrs: { start: 1 }, content: listItems }
    : { type: 'bulletList', content: listItems };
}

/** Markdown → document, for the mock. */
export function markdownToDoc(markdown: string): NoteDoc {
  const lines = markdown
    .split('\n')
    .filter((line) => line.trim() !== '')
    .map(parseLine);
  const content: Block[] = [];
  let index = 0;
  while (index < lines.length) {
    const line = lines[index];
    if (line === undefined) break;
    if (line.kind === 'heading') {
      content.push({
        type: 'heading',
        attrs: { level: Math.min(line.level, 3) },
        content: inlines(line.text),
      });
      index += 1;
    } else if (line.kind === 'text') {
      content.push({ type: 'paragraph', content: inlines(line.text) });
      index += 1;
    } else {
      const [block, after] = list(lines, index);
      content.push(block);
      index = after;
    }
  }
  return {
    type: 'doc',
    content: content.length > 0 ? content : [{ type: 'paragraph', content: [] }],
  };
}

function inlineMarkdown(content: readonly Inline[]): string {
  return content
    .map((inline) => {
      if (inline.type === 'hardBreak') return '\n';
      if (inline.type === 'mention') return `@${inline.attrs.label}`;
      if (inline.type === 'citation') return '';
      const types = (inline.marks ?? []).map((mark) => mark.type);
      if (types.includes('code')) return `\`${inline.text}\``;
      const bold = types.includes('bold') ? '**' : '';
      const italic = types.includes('italic') ? '*' : '';
      return `${bold}${italic}${inline.text}${italic}${bold}`;
    })
    .join('');
}

/** List items, each its marker then its blocks, nested blocks indented. */
function itemsMarkdown(
  indent: string,
  prefix: (index: number, checked: boolean) => string,
  entries: readonly { content: Block[]; checked: boolean }[],
): string {
  return entries
    .map(({ content, checked }, index) =>
      content
        .map((child, at) =>
          at === 0
            ? `${indent}${prefix(index, checked)}${blockMarkdown(child, '')}`
            : blockMarkdown(child, `${indent}  `),
        )
        .join('\n'),
    )
    .join('\n');
}

function blockMarkdown(block: Block, indent: string): string {
  const items = (
    prefix: (index: number, checked: boolean) => string,
    entries: readonly { content: Block[]; checked: boolean }[],
  ) => itemsMarkdown(indent, prefix, entries);
  switch (block.type) {
    case 'heading':
      return `${indent}${'#'.repeat(block.attrs.level)} ${inlineMarkdown(block.content ?? [])}`;
    case 'paragraph':
      return `${indent}${inlineMarkdown(block.content ?? [])}`;
    case 'bulletList':
      return items(
        () => '- ',
        block.content.map((item) => ({ content: item.content, checked: false })),
      );
    case 'orderedList':
      return items(
        (index) => `${String(index + 1)}. `,
        block.content.map((item) => ({ content: item.content, checked: false })),
      );
    case 'taskList':
      return items(
        (_, checked) => (checked ? '- [x] ' : '- [ ] '),
        block.content.map((item) => ({
          content: item.content,
          checked: item.attrs.checked ?? false,
        })),
      );
    default:
      return '';
  }
}

/** Document → Markdown, for the mock. */
export function docToMarkdown(doc: NoteDoc): string {
  const text = doc.content
    .map((block) => blockMarkdown(block, ''))
    .filter((part) => part !== '')
    .join('\n\n');
  return text === '' ? '' : `${text}\n`;
}
