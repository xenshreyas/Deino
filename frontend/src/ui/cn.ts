/**
 * `cn`: joins class names and resolves Tailwind conflicts (last one wins).
 *
 * tailwind-merge is taught the design system's custom scales, so a
 * `shadow-popover` replaces a `shadow-xs` instead of being read as a shadow
 * colour, and the type scale and token colours stay distinct.
 */
import { type ClassValue, clsx } from 'clsx';
import { extendTailwindMerge } from 'tailwind-merge';

const merge = extendTailwindMerge({
  extend: {
    theme: {
      shadow: ['xs', 'popover', 'dialog', 'chatbar', 'pill'],
      text: ['xs', 'sm', 'base', 'lg', 'xl', '2xl', '3xl'],
    },
  },
});

/** Combines conditional class names; later Tailwind utilities override earlier ones. */
export function cn(...inputs: ClassValue[]): string {
  return merge(clsx(inputs));
}
