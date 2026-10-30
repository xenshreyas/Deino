/**
 * A note document drawn as the editor draws it, without the editor: the
 * notes while they are written (streamed notes). Same elements and
 * attributes as TipTap's (`h3`, `li > p`, `ul[data-type=taskList]`,
 * `span[data-origin]`, mention chips), inside the same `.note-content
 * .enhanced-notes` styles, so the saved version replaces it without a
 * visible change. Citations are hidden in the notes, so they are left out.
 * Pure: React keeps each line's element as later lines arrive, which is
 * what lets only new lines fade in.
 */
import type { ReactNode } from 'react';

import type { Block, DraftDoc, Inline, Mark } from '../types';

/** The blocks of `doc`, top to bottom. */
export function DocView({ doc }: { doc: DraftDoc }) {
  return <Blocks blocks={doc.content} />;
}

function Blocks({ blocks }: { blocks: readonly Block[] }) {
  return (
    <>
      {blocks.map((block, index) => (
        <BlockView key={index} block={block} />
      ))}
    </>
  );
}

function BlockView({ block }: { block: Block }): ReactNode {
  switch (block.type) {
    case 'paragraph':
      return <p>{inlines(block.content)}</p>;
    case 'heading':
      return <Heading level={block.attrs.level}>{inlines(block.content)}</Heading>;
    case 'bulletList':
    case 'orderedList':
      return <List ordered={block.type === 'orderedList'} items={block.content} />;
    case 'taskList':
      return (
        <ul data-type="taskList">
          {block.content.map((item, index) => (
            <TaskRow key={index} checked={item.attrs.checked === true} blocks={item.content} />
          ))}
        </ul>
      );
    case 'blockquote':
      return (
        <blockquote>
          <Blocks blocks={block.content} />
        </blockquote>
      );
    case 'codeBlock':
      return (
        <pre>
          <code>{inlines(block.content)}</code>
        </pre>
      );
    case 'horizontalRule':
      return <hr />;
    case 'image':
      return null;
  }
}

function Heading({ level, children }: { level: number; children: ReactNode }) {
  if (level <= 1) return <h1>{children}</h1>;
  if (level === 2) return <h2>{children}</h2>;
  return <h3>{children}</h3>;
}

function List({ ordered, items }: { ordered: boolean; items: { content: Block[] }[] }) {
  const rows = items.map((item, index) => (
    <li key={index}>
      <Blocks blocks={item.content} />
    </li>
  ));
  return ordered ? <ol>{rows}</ol> : <ul>{rows}</ul>;
}

function TaskRow({ checked, blocks }: { checked: boolean; blocks: Block[] }) {
  return (
    <li data-type="taskItem" data-checked={String(checked)}>
      <label>
        <input type="checkbox" checked={checked} disabled readOnly tabIndex={-1} />
        <span />
      </label>
      <div>
        <Blocks blocks={blocks} />
      </div>
    </li>
  );
}

function inlines(content: readonly Inline[] | undefined): ReactNode {
  return (content ?? []).map((inline, index) => <InlineView key={index} inline={inline} />);
}

function InlineView({ inline }: { inline: Inline }): ReactNode {
  switch (inline.type) {
    case 'text':
      return marked(inline.text, inline.marks ?? []);
    case 'hardBreak':
      return <br />;
    case 'mention':
      return marked(<Mention label={inline.attrs.label} />, inline.marks ?? []);
    case 'citation':
      return null;
  }
}

function Mention({ label }: { label: string }) {
  const initials = label
    .split(/\s+/)
    .filter((part) => part !== '')
    .map((part) => part.charAt(0))
    .slice(0, 2)
    .join('')
    .toUpperCase();
  return (
    <span className="note-mention" data-type="mention" data-label={label}>
      <span className="note-mention-avatar" aria-hidden="true">
        {initials}
      </span>
      {label}
    </span>
  );
}

/** `content` inside each of its marks, outermost first. */
function marked(content: ReactNode, marks: readonly Mark[]): ReactNode {
  return marks.reduceRight<ReactNode>((inner, mark) => wrap(inner, mark), content);
}

function wrap(inner: ReactNode, mark: Mark): ReactNode {
  switch (mark.type) {
    case 'origin':
      return <span data-origin={mark.attrs.author}>{inner}</span>;
    case 'link':
      return (
        <a href={mark.attrs.href} target="_blank" rel="noreferrer">
          {inner}
        </a>
      );
    case 'bold':
      return <strong>{inner}</strong>;
    case 'italic':
      return <em>{inner}</em>;
    case 'underline':
      return <u>{inner}</u>;
    case 'strike':
      return <s>{inner}</s>;
    case 'code':
      return <code>{inner}</code>;
  }
}
