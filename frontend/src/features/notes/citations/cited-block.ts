/**
 * Which block of the rendered notes a pointer or the caret is in, and the
 * citations that block carries (NOT-03).
 *
 * A block is a list item (its own line only, not its nested sub-points,
 * which have their own citations) or a paragraph outside a list. Its
 * citations are the `citation` nodes `deino-doc` placed after its text,
 * rendered as `span[data-segment-id]` by the notepad's citation node.
 */
import type { CitationRef } from '../types';

/** A block with sources, and where it sits. */
export interface CitedBlock {
  element: HTMLElement;
  citations: CitationRef[];
}

/** The block element `node` belongs to inside `root`, if any. */
function blockOf(node: Node | null, root: HTMLElement): HTMLElement | null {
  const element = node instanceof HTMLElement ? node : (node?.parentElement ?? null);
  const block = element?.closest<HTMLElement>('li, p') ?? null;
  if (block === null || !root.contains(block)) return null;
  if (block.tagName === 'P') return block.closest<HTMLElement>('li') ?? block;
  return block;
}

/** The paragraph that is the block's own line. */
function ownLine(block: HTMLElement): HTMLElement | null {
  if (block.tagName === 'P') return block;
  return block.querySelector<HTMLElement>(':scope > p, :scope > div > p');
}

/** The citations on `line`, in order, without repeats. */
function citationsOf(line: HTMLElement): CitationRef[] {
  const seen = new Set<string>();
  const refs: CitationRef[] = [];
  for (const chip of line.querySelectorAll<HTMLElement>('[data-segment-id]')) {
    const segmentId = chip.dataset.segmentId ?? '';
    if (segmentId === '' || seen.has(segmentId)) continue;
    seen.add(segmentId);
    refs.push({ segmentId, startMs: Number(chip.dataset.startMs ?? 0) });
  }
  return refs;
}

/** The cited block containing `node`; `null` for a block without sources. */
export function citedBlockAt(node: Node | null, root: HTMLElement): CitedBlock | null {
  const element = blockOf(node, root);
  const line = element === null ? null : ownLine(element);
  if (element === null || line === null) return null;
  const citations = citationsOf(line);
  return citations.length === 0 ? null : { element, citations };
}

/** Where `element`'s first line sits inside `frame`, in px from its top. */
export function offsetWithin(element: HTMLElement, frame: HTMLElement): number {
  const line = ownLine(element) ?? element;
  return line.getBoundingClientRect().top - frame.getBoundingClientRect().top;
}
