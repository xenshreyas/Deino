/**
 * Deino's brand (BRAND-01): the mark and the wordmark, each a span painted
 * with a design token through a CSS mask over the committed alpha PNGs
 * (`scripts/brand/make-brand-assets.py` writes `ui/brand-masks/`). The mark
 * takes `--brand`, the wordmark `--fg`, so both follow the theme.
 */
import type { CSSProperties } from 'react';

import markMask from './brand-masks/mark.png';
import wordmarkMask from './brand-masks/wordmark.png';
import { cn } from './cn';

/** Width over height of each mask (their pixel sizes), so a height sizes them. */
const MARK_ASPECT = 206 / 256;
const WORDMARK_ASPECT = 562 / 171;

/** The accessible name of both: the product. */
const BRAND_NAME = 'Deino';

/** The inline style of a masked span `height` pixels tall. */
function maskStyle(mask: string, aspect: number, height: number): CSSProperties {
  return {
    height,
    width: height * aspect,
    maskImage: `url("${mask}")`,
    maskSize: 'contain',
    maskRepeat: 'no-repeat',
    maskPosition: 'center',
  };
}

/** Props of {@link BrandMark}. */
export interface BrandMarkProps {
  /** Height in pixels; the width follows. 16 by default. */
  size?: number;
  /**
   * Set when the mark stands alone and names the app (`role="img"`); left
   * out, the mark is decorative because text beside it says "Deino".
   */
  label?: string;
  className?: string;
}

/** The mark (the creature's head) in the brand blue. */
export function BrandMark({ size = 16, label, className }: BrandMarkProps) {
  const name = label === undefined ? { 'aria-hidden': true } : { role: 'img', 'aria-label': label };
  return (
    <span
      {...name}
      className={cn('inline-block shrink-0 bg-brand', className)}
      style={maskStyle(markMask, MARK_ASPECT, size)}
    />
  );
}

/** Props of {@link Wordmark}. */
export interface WordmarkProps {
  /** Height in pixels (of the letters' bounds); the width follows. 14 by default. */
  size?: number;
  className?: string;
}

/** "deino" in the brand's own letters, in the foreground colour. */
export function Wordmark({ size = 14, className }: WordmarkProps) {
  return (
    <span
      role="img"
      aria-label={BRAND_NAME}
      className={cn('inline-block shrink-0 bg-fg', className)}
      style={maskStyle(wordmarkMask, WORDMARK_ASPECT, size)}
    />
  );
}
