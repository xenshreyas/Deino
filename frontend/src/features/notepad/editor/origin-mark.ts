/**
 * The `origin` mark (`deino_doc::Mark::Origin`): who wrote a run of text.
 * NOT-02 sets it on enhanced notes; the notepad keeps it intact and styles
 * user text in `--user-text` and AI text in `--ai-text` (ui.md §1 rule 3).
 * Registered first, so it ranks outermost, as in `deino-doc`.
 */
import { Mark, mergeAttributes } from '@tiptap/core';

/** Provenance over inline content. */
export const OriginMark = Mark.create({
  name: 'origin',

  addAttributes() {
    return {
      author: {
        default: 'user',
        parseHTML: (element) => element.getAttribute('data-origin'),
        renderHTML: (attributes) => ({ 'data-origin': String(attributes.author) }),
      },
      edited: {
        default: false,
        parseHTML: (element) => element.getAttribute('data-edited') === 'true',
        renderHTML: (attributes) => (attributes.edited === true ? { 'data-edited': 'true' } : {}),
      },
    };
  },

  parseHTML() {
    return [{ tag: 'span[data-origin]' }];
  },

  renderHTML({ HTMLAttributes }) {
    return ['span', mergeAttributes({ class: 'note-origin' }, HTMLAttributes), 0];
  },
});
