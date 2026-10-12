/**
 * Text helpers: what a streaming answer shows before its citations resolve,
 * an answer without citations (for copying), and the `/recipe` the user is
 * typing. Pure, tested on their own.
 */
import type { AnswerPart } from './types';

const REFERENCE = String.raw`[sS]\d+(?:\s+\d{1,2}:\d{2}(?::\d{2})?)?`;
/** One `[s12]`, `[s3, s4]` or `[s7 04:31]` marker, with the space before it. */
const MARKER = new RegExp(String.raw`\s*\[\s*${REFERENCE}(?:\s*[,;]\s*${REFERENCE})*\s*\]`, 'g');
/** A marker still arriving at the end of a stream (`… [s1`). */
const PARTIAL = /\s*\[[sS]?[\d\s:,;sS]*$/;

/** A streaming answer without its markers, complete or half-arrived. */
export function streamingText(text: string): string {
  return text.replace(MARKER, '').replace(PARTIAL, '');
}

/** An answer's text without citations, as Copy puts it on the clipboard. */
export function plainText(parts: readonly AnswerPart[]): string {
  return parts
    .map((part) => part.text)
    .join('')
    .trim();
}

/** What the user is typing after `/` (`"/foll"` → `"foll"`), or `null`. */
export function slashQuery(input: string): string | null {
  const match = /^\/([a-z0-9-]*)$/i.exec(input);
  return match === null ? null : (match[1] ?? '').toLowerCase();
}

/** A typed `/slug rest` split into the slug and the rest, or `null`. */
export function typedRecipe(input: string): { slug: string; rest: string } | null {
  const match = /^\/([a-z0-9-]+)(?:\s+([\s\S]*))?$/i.exec(input.trim());
  if (match === null) return null;
  return { slug: (match[1] ?? '').toLowerCase(), rest: (match[2] ?? '').trim() };
}
