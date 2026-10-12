import { describe, expect, it } from 'vitest';

import { selectedTextblocks, type TextblockNode } from './selection';

/** A text block of `size` at the position the walk reports. */
function textblock(size: number): TextblockNode {
  return { isTextblock: true, nodeSize: size, descendants: () => undefined };
}

/** A container whose descendants are `children` at `positions`. */
function container(children: [TextblockNode, number][]): TextblockNode {
  return {
    isTextblock: false,
    nodeSize: 0,
    descendants: (visit) => {
      for (const [node, pos] of children) visit(node, pos);
    },
  };
}

// heading 0–10, list item paragraphs 12–20 and 22–30, paragraph 32–40.
const DOC = container([
  [textblock(10), 0],
  [textblock(8), 12],
  [textblock(8), 22],
  [textblock(8), 32],
]);

describe('selectedTextblocks', () => {
  it('numbers the text blocks a range touches, as Rust numbers them', () => {
    expect(selectedTextblocks(DOC, 15, 25)).toEqual([1, 2]);
    expect(selectedTextblocks(DOC, 1, 40)).toEqual([0, 1, 2, 3]);
  });

  it('a caret is no selection: the whole notes', () => {
    expect(selectedTextblocks(DOC, 15, 15)).toEqual([]);
  });
});
