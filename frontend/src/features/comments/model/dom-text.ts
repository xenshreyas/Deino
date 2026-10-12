/**
 * Character offsets in an element's text, as the webview counts them (UTF-16
 * code units of `textContent`), mapped to and from DOM positions. The
 * transcript counts in one bubble; the notes in the whole document.
 */
import type { Span } from './anchor';

/** The offset of DOM position (`node`, `offset`) in `root`'s text, or `null` outside it. */
export function textOffset(root: Node, node: Node, offset: number): number | null {
  if (!root.contains(node)) return null;
  const range = document.createRange();
  range.selectNodeContents(root);
  range.setEnd(node, offset);
  return range.toString().length;
}

/** The DOM position of text offset `at` in `root`: a text node and an offset in it. */
function positionAt(root: Node, at: number): { node: Text; offset: number } | null {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  let seen = 0;
  for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) {
    const length = node.textContent?.length ?? 0;
    if (at <= seen + length && node instanceof Text) return { node, offset: at - seen };
    seen += length;
  }
  return null;
}

/** A DOM range over `span` of `root`'s text, or `null` when the text is shorter. */
export function rangeOf(root: Node, span: Span): Range | null {
  const start = positionAt(root, span.start);
  const end = positionAt(root, span.end);
  if (start === null || end === null) return null;
  const range = document.createRange();
  range.setStart(start.node, start.offset);
  range.setEnd(end.node, end.offset);
  return range;
}
