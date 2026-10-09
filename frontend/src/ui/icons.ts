/**
 * The app's icons, the only module that imports an icon library (ESLint
 * enforces it). Lucide at 16 px with a 1.5 stroke is the house style (§3B);
 * add an icon here before using it.
 */
export { DatabaseZap, NotebookPen, Plus } from 'lucide-react';

/** Props that give every icon the house size and stroke. */
export const ICON_PROPS = { size: 16, strokeWidth: 1.5, 'aria-hidden': true } as const;
