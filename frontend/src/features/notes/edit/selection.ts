/**
 * The selected text blocks of a ProseMirror document, numbered as Rust
 * numbers them (`deino_core::notes::edit`): every paragraph, heading and
 * code block in document order, from 0.
 */

/** The parts of a ProseMirror node the walk needs. */
export interface TextblockNode {
  isTextblock: boolean;
  nodeSize: number;
  descendants: (visit: (node: TextblockNode, pos: number) => boolean | undefined) => void;
}

/**
 * The positions of the text blocks the range `from`–`to` touches; empty
 * for a collapsed selection (a caret edits the whole notes).
 */
export function selectedTextblocks(doc: TextblockNode, from: number, to: number): number[] {
  if (from === to) return [];
  const selected: number[] = [];
  let index = 0;
  doc.descendants((node, pos) => {
    if (!node.isTextblock) return true;
    const end = pos + node.nodeSize;
    if (pos < to && end > from) selected.push(index);
    index += 1;
    return false;
  });
  return selected;
}
