/**
 * Images in notes (`deino_doc::Block::Image`). The document keeps the stored
 * source (`images/<id>.png`, relative to the meeting's folder); only the
 * rendered `<img>` gets the displayable URL, and the source rides along as
 * `data-src`, so copy and paste inside the app keeps the reference. Images
 * from anywhere else (HTML pasted from the web) are not parsed: the notepad
 * stores pasted image files instead (`useImageInsert`).
 */
import { mergeAttributes, Node } from '@tiptap/core';

/** Options of {@link NoteImage}. */
export interface NoteImageOptions {
  /** Turns a stored source into a URL the webview can load. */
  resolveSrc: (src: string) => string;
}

/** True for a source the notes service stored (`images/<uuid>.<ext>`). */
export function isStoredImageSrc(src: string): boolean {
  return /^images\/[0-9a-f-]{36}\.(png|jpg|gif|webp)$/.test(src);
}

/** A block image, rendered through `resolveSrc`. */
export const NoteImage = Node.create<NoteImageOptions>({
  name: 'image',
  group: 'block',
  atom: true,
  draggable: true,

  addOptions() {
    return { resolveSrc: (src: string) => src };
  },

  addAttributes() {
    return {
      src: { default: null, parseHTML: (element) => element.getAttribute('data-src') },
      alt: { default: null },
      title: { default: null },
    };
  },

  parseHTML() {
    return [
      {
        tag: 'img[data-src]',
        getAttrs: (element) =>
          isStoredImageSrc(element.getAttribute('data-src') ?? '') ? null : false,
      },
    ];
  },

  renderHTML({ node, HTMLAttributes }) {
    const src = typeof node.attrs.src === 'string' ? node.attrs.src : '';
    return [
      'img',
      mergeAttributes(HTMLAttributes, {
        class: 'note-image',
        src: this.options.resolveSrc(src),
        'data-src': src,
      }),
    ];
  },
});
