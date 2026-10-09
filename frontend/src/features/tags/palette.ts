/**
 * Tag colours: the six `--speaker-N` slots (ui.md §3 Tag pill: tag colours
 * reuse the speaker palette), as literal utility classes so Tailwind keeps
 * them, with a name for screen readers.
 */
import type { PaletteColor } from './types';

/** One palette slot. */
export interface PaletteSlot {
  color: PaletteColor;
  name: string;
  /** The dot's background utility. */
  dot: string;
}

/** Every slot, in palette order. */
export const PALETTE: readonly PaletteSlot[] = [
  { color: 1, name: 'Blue', dot: 'bg-speaker-1' },
  { color: 2, name: 'Violet', dot: 'bg-speaker-2' },
  { color: 3, name: 'Teal', dot: 'bg-speaker-3' },
  { color: 4, name: 'Amber', dot: 'bg-speaker-4' },
  { color: 5, name: 'Rose', dot: 'bg-speaker-5' },
  { color: 6, name: 'Olive', dot: 'bg-speaker-6' },
];

/** The slot of `color`; an unknown slot falls back to the first. */
export function paletteSlot(color: PaletteColor): PaletteSlot {
  const slot = PALETTE.find((candidate) => candidate.color === color) ?? PALETTE[0];
  if (slot === undefined) throw new Error('The tag palette is empty.');
  return slot;
}
