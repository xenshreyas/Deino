/**
 * The Markdown of a stored summary, whatever format saved it.
 *
 * Summaries are Markdown since the Rust generator wrote them; older rows
 * hold BlockNote blocks (`summary_json`, saved by the BlockNote editor this
 * card removed) or the first app's section JSON (`{ Key: { title, blocks } }`).
 * The summary view edits Markdown only, so both are read once into it, and
 * the next save stores Markdown.
 */
import { z } from 'zod';

/** BlockNote's inline content: styled text and links. */
const blockNoteInline: z.ZodType<BlockNoteInline> = z.lazy(() =>
  z.object({
    type: z.string(),
    text: z.string().optional(),
    href: z.string().optional(),
    styles: z.record(z.unknown()).optional(),
    content: z.array(blockNoteInline).optional(),
  }),
);

interface BlockNoteInline {
  type: string;
  text?: string | undefined;
  href?: string | undefined;
  styles?: Record<string, unknown> | undefined;
  content?: BlockNoteInline[] | undefined;
}

interface BlockNoteBlock {
  type: string;
  props?: Record<string, unknown> | undefined;
  content?: BlockNoteInline[] | string | undefined;
  children?: BlockNoteBlock[] | undefined;
}

const blockNoteBlock: z.ZodType<BlockNoteBlock> = z.lazy(() =>
  z.object({
    type: z.string(),
    props: z.record(z.unknown()).optional(),
    content: z.union([z.array(blockNoteInline), z.string()]).optional(),
    children: z.array(blockNoteBlock).optional(),
  }),
);

const storedSummary = z.object({
  markdown: z.string().optional(),
  summary_json: z.array(blockNoteBlock).optional(),
});

const legacySection = z.object({
  title: z.string().optional(),
  blocks: z.array(z.object({ type: z.string(), content: z.string() })),
});

/** Wraps text in the Markdown of BlockNote's styles. */
function styled(text: string, styles: Record<string, unknown> | undefined): string {
  if (styles === undefined || text.trim() === '') return text;
  let out = text;
  if (styles.code === true) out = `\`${out}\``;
  if (styles.bold === true) out = `**${out}**`;
  if (styles.italic === true) out = `*${out}*`;
  if (styles.strike === true) out = `~~${out}~~`;
  return out;
}

function inlineMarkdown(content: BlockNoteBlock['content']): string {
  if (typeof content === 'string') return content;
  return (content ?? [])
    .map((item) =>
      item.type === 'link'
        ? `[${inlineMarkdown(item.content)}](${item.href ?? ''})`
        : styled(item.text ?? '', item.styles),
    )
    .join('');
}

/** The Markdown line(s) of one BlockNote block and its nested children. */
function blockMarkdown(block: BlockNoteBlock, indent: string): string {
  const text = inlineMarkdown(block.content);
  const level = typeof block.props?.level === 'number' ? block.props.level : 1;
  const prefixes: Readonly<Record<string, string>> = {
    heading: `${'#'.repeat(level)} `,
    bulletListItem: '- ',
    numberedListItem: '1. ',
    checkListItem: block.props?.checked === true ? '- [x] ' : '- [ ] ',
  };
  const line = `${indent}${prefixes[block.type] ?? ''}${text}`;
  const children = (block.children ?? []).map((child) => blockMarkdown(child, `${indent}  `));
  return [line, ...children].join('\n');
}

/** BlockNote blocks as Markdown; list items stay tight, other blocks get blank lines. */
export function blockNoteToMarkdown(blocks: readonly BlockNoteBlock[]): string {
  return blocks
    .map((block, index) => {
      const isItem = block.type.endsWith('ListItem');
      const previousIsItem = blocks[index - 1]?.type.endsWith('ListItem') === true;
      const separator = index === 0 ? '' : isItem && previousIsItem ? '\n' : '\n\n';
      return separator + blockMarkdown(block, '');
    })
    .join('');
}

/** The first app's section JSON as Markdown, one `##` section each. */
export function legacySectionsToMarkdown(summary: Record<string, unknown>): string {
  const order = z.array(z.string()).safeParse(summary._section_order);
  const keys = order.success ? order.data : Object.keys(summary);
  const sections = keys.flatMap((key) => {
    const section = legacySection.safeParse(summary[key]);
    if (!section.success) return [];
    const lines = section.data.blocks.map(({ type, content }) =>
      type === 'bullet' ? `- ${content}` : type.startsWith('heading') ? `### ${content}` : content,
    );
    return [`## ${section.data.title ?? key}\n\n${lines.join('\n')}`];
  });
  return sections.join('\n\n');
}

/** The summary's Markdown, from any stored format; empty when there is none. */
export function summaryMarkdown(summary: unknown): string {
  const stored = storedSummary.safeParse(summary);
  if (stored.success && stored.data.markdown !== undefined) return stored.data.markdown;
  if (stored.success && stored.data.summary_json !== undefined) {
    return blockNoteToMarkdown(stored.data.summary_json);
  }
  const record = z.record(z.unknown()).safeParse(summary);
  return record.success ? legacySectionsToMarkdown(record.data) : '';
}
