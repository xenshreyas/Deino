/**
 * The `citation` inline atom (`deino_doc::Inline::Citation`): a transcript
 * moment backing the text before it, shown as a small timestamp chip.
 * NOT-02 writes citations and NOT-03 makes them open the transcript; the
 * notepad renders and preserves them.
 */
import { mergeAttributes, Node } from '@tiptap/core';

/** `m:ss`, or `h:mm:ss` from an hour on, like `deino-doc`'s Readable Markdown. */
export function formatTimestamp(ms: number): string {
  const total = Math.floor(ms / 1000);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor(total / 60) % 60;
  const seconds = String(total % 60).padStart(2, '0');
  return hours > 0
    ? `${String(hours)}:${String(minutes).padStart(2, '0')}:${seconds}`
    : `${String(minutes)}:${seconds}`;
}

/** A citation of one transcript segment. */
export const CitationNode = Node.create({
  name: 'citation',
  group: 'inline',
  inline: true,
  atom: true,
  selectable: true,

  addAttributes() {
    return {
      segmentId: {
        default: null,
        parseHTML: (element) => element.getAttribute('data-segment-id'),
        renderHTML: (attributes) => ({ 'data-segment-id': String(attributes.segmentId) }),
      },
      startMs: {
        default: 0,
        parseHTML: (element) => Number(element.getAttribute('data-start-ms')),
        renderHTML: (attributes) => ({ 'data-start-ms': String(attributes.startMs) }),
      },
    };
  },

  parseHTML() {
    return [{ tag: 'span[data-segment-id]' }];
  },

  renderHTML({ node, HTMLAttributes }) {
    const startMs = typeof node.attrs.startMs === 'number' ? node.attrs.startMs : 0;
    return [
      'span',
      mergeAttributes({ class: 'note-citation' }, HTMLAttributes),
      formatTimestamp(startMs),
    ];
  },

  renderText({ node }) {
    const startMs = typeof node.attrs.startMs === 'number' ? node.attrs.startMs : 0;
    return `[${formatTimestamp(startMs)}]`;
  },
});
