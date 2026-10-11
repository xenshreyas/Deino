/**
 * Marks AI text the user edits as edited (NOT-02): it keeps its grey but
 * its `origin` mark gains `edited: true`, so the notes always tell what the
 * AI wrote untouched from what a person changed.
 *
 * After every change, each text run the change touched (one position
 * either side, so a deletion inside a run counts) that carries an AI
 * `origin` not yet edited gets `edited` set, in one appended transaction.
 * Text typed inside an AI run inherits its mark (marks are inclusive), so
 * it is edited AI text too; text typed in a new paragraph has no origin and
 * reads as the user's.
 */
import { Extension } from '@tiptap/core';
import type { Node as ProseMirrorNode } from '@tiptap/pm/model';
import { Plugin, PluginKey, type Transaction } from '@tiptap/pm/state';

/** The meta set on this plugin's own transactions, so it never loops. */
const META = 'editedAiText';

/** A range of the new document a change touched. */
interface Range {
  from: number;
  to: number;
}

/** Every range `transactions` changed, mapped into the final document. */
export function changedRanges(transactions: readonly Transaction[], size: number): Range[] {
  const maps = transactions.flatMap((transaction) => transaction.mapping.maps);
  const ranges: Range[] = [];
  maps.forEach((map, index) => {
    map.forEach((_oldStart, _oldEnd, newStart, newEnd) => {
      let from = newStart;
      let to = newEnd;
      for (const later of maps.slice(index + 1)) {
        from = later.map(from, -1);
        to = later.map(to, 1);
      }
      ranges.push({ from: Math.max(0, from - 1), to: Math.min(size, to + 1) });
    });
  });
  return ranges;
}

/** Sets `edited` on unedited AI runs in `ranges`; `null` when none. */
function markEdited(doc: ProseMirrorNode, ranges: Range[], tr: Transaction): Transaction | null {
  const origin = doc.type.schema.marks.origin;
  if (origin === undefined) return null;
  const runs: { from: number; to: number; attrs: Record<string, unknown> }[] = [];
  for (const { from, to } of ranges) {
    doc.nodesBetween(from, to, (node, pos) => {
      const mark = node.marks.find(
        (candidate) => candidate.type === origin && candidate.attrs.author === 'ai',
      );
      if (mark !== undefined && mark.attrs.edited !== true) {
        runs.push({ from: pos, to: pos + node.nodeSize, attrs: mark.attrs });
      }
      return true;
    });
  }
  if (runs.length === 0) return null;
  for (const run of runs) {
    tr.addMark(run.from, run.to, origin.create({ ...run.attrs, edited: true }));
  }
  return tr.setMeta(META, true);
}

/** The extension; add it to an enhanced-notes editor. */
export const EditedAiText = Extension.create({
  name: 'editedAiText',

  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: new PluginKey(META),
        appendTransaction: (transactions, _old, state) => {
          const edits = transactions.filter((tr) => tr.docChanged && tr.getMeta(META) !== true);
          if (edits.length === 0) return null;
          return markEdited(state.doc, changedRanges(edits, state.doc.content.size), state.tr);
        },
      }),
    ];
  },
});
