/**
 * @mentions of people (`deino_doc::Inline::Mention`): typing `@` opens a
 * menu of people from the `people` plugin; choosing one inserts a person
 * chip (ui.md §3 Speaker chip, "inline in notes"). The menu itself is React
 * (`MentionMenu`), driven through a {@link MentionStore}.
 */
import { mergeAttributes } from '@tiptap/core';
import Mention from '@tiptap/extension-mention';

import type { MentionCandidate } from '../types';
import { type MentionStore, suggestionRenderer } from './menu-store';

/** How long typing pauses before the people search runs, in ms. */
const SEARCH_DEBOUNCE_MS = 120;

/** Finds the people to offer for a query. */
export type MentionSearch = (query: string) => Promise<MentionCandidate[]>;

/** Up to two initials for a chip's avatar. */
export function initials(name: string): string {
  const words = name.trim().split(/\s+/).filter(Boolean);
  const first = words[0]?.[0] ?? '';
  const last = words.length > 1 ? (words.at(-1)?.[0] ?? '') : '';
  return `${first}${last}`.toUpperCase();
}

/**
 * The mention node. Without a store (a document such as the enhanced
 * notes) chips render and survive edits, but `@` opens nothing.
 */
export function mentionExtension(store: MentionStore | null, search: MentionSearch) {
  return Mention.configure({
    renderText: ({ node }) => `@${String(node.attrs.label ?? '')}`,
    renderHTML: ({ node }) => {
      const label = String(node.attrs.label ?? '');
      return [
        'span',
        mergeAttributes({
          class: 'note-mention',
          'data-type': 'mention',
          'data-id': String(node.attrs.id ?? ''),
          'data-label': label,
        }),
        ['span', { class: 'note-mention-avatar', 'aria-hidden': 'true' }, initials(label)],
        label,
      ];
    },
    suggestion: {
      char: '@',
      debounce: SEARCH_DEBOUNCE_MS,
      allow: () => store !== null,
      items: ({ query }) => search(query),
      render: () => suggestionBridge(store),
    },
  });
}

/** What the mention plugin hands the menu. */
interface MentionRenderProps {
  items: MentionCandidate[];
  clientRect?: (() => DOMRect | null) | null;
  command: (attrs: { id: string; label: string }) => void;
}

/** Drives the menu's store; choosing inserts the person's chip. */
function suggestionBridge(store: MentionStore | null) {
  return suggestionRenderer<MentionCandidate, MentionRenderProps>(store, (props, candidate) => {
    props.command({ id: candidate.id, label: candidate.label });
  });
}
