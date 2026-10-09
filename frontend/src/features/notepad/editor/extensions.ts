/**
 * The editor's schema: TipTap 3's StarterKit (with its Markdown input rules:
 * `#` headings, `-`/`*` bullets, `1.` numbers, `>` quotes, `**bold**`,
 * `*italic*`, `~~strike~~`, `` `code` ``), checklists (`[] ` and `[x] `),
 * the placeholder, the `/template` command (NOT-04), and Deino's nodes:
 * provenance, citations, mentions and stored images. Every node and mark here has a twin in `deino-doc`.
 */
import type { Extensions } from '@tiptap/core';
import { TaskItem, TaskList } from '@tiptap/extension-list';
import { Placeholder } from '@tiptap/extensions';
import StarterKit from '@tiptap/starter-kit';

import { CitationNode } from './citation-node';
import { type MentionSearch, mentionExtension } from './mention';
import type { MentionStore } from './menu-store';
import { NoteImage } from './note-image';
import { OriginMark } from './origin-mark';
import { PlainTextPaste } from './plain-paste';
import { TemplateCommand, type TemplateCommandOptions } from './template-command';

/** Link schemes the notepad keeps; matches `deino_doc::is_safe_href`. */
const SAFE_LINK = /^(https?:\/\/|mailto:)/i;

/** What an editor instance needs from its surroundings. */
export interface NoteExtensionOptions {
  /** The empty document's hint. */
  placeholder: string;
  /** Turns a stored image source into a loadable URL. */
  resolveImageSrc: (src: string) => string;
  /** The @mention menu; `null` turns mentions off (they still render). */
  mentions: MentionStore | null;
  /** Finds people for the menu. */
  searchPeople: MentionSearch;
  /** The `/template` command's menu and actions; nulls turn it off. */
  templates: TemplateCommandOptions;
}

/** The extensions of a note editor. */
export function noteExtensions(options: NoteExtensionOptions): Extensions {
  return [
    // First, so provenance ranks outermost (as `deino_doc::Mark::rank`).
    OriginMark,
    StarterKit.configure({
      heading: { levels: [1, 2, 3] },
      link: {
        openOnClick: false,
        autolink: true,
        defaultProtocol: 'https',
        isAllowedUri: (url) => SAFE_LINK.test(url),
      },
      dropcursor: { color: 'var(--accent)', width: 2 },
    }),
    TaskList,
    TaskItem.configure({ nested: true }),
    Placeholder.configure({ placeholder: options.placeholder }),
    CitationNode,
    NoteImage.configure({ resolveSrc: options.resolveImageSrc }),
    mentionExtension(options.mentions, options.searchPeople),
    TemplateCommand.configure(options.templates),
    PlainTextPaste,
  ];
}
