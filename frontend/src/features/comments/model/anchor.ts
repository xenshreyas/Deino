/**
 * Finding a commented span again (TRN-06, ADR 0048). A thread stores the
 * span's offsets and its text; after an edit the text may have moved or
 * gone. The span is where the offsets point if the quote is still there,
 * else the occurrence of the quote nearest the old start, else nowhere:
 * the thread is detached (kept, shown without a highlight).
 */

/** A character range, end exclusive. */
export interface Span {
  start: number;
  end: number;
}

/** What a stored anchor says about its span. */
export interface QuotedSpan extends Span {
  quote: string;
}

/** Every index where `quote` starts in `text`. */
function occurrences(text: string, quote: string): number[] {
  const found: number[] = [];
  for (let at = text.indexOf(quote); at >= 0; at = text.indexOf(quote, at + 1)) {
    found.push(at);
  }
  return found;
}

/** The span of `anchor` in `text` now, or `null` when its quote is gone. */
export function locateQuote(text: string, anchor: QuotedSpan): Span | null {
  if (anchor.quote === '') return null;
  if (text.slice(anchor.start, anchor.end) === anchor.quote) {
    return { start: anchor.start, end: anchor.end };
  }
  const starts = occurrences(text, anchor.quote);
  if (starts.length === 0) return null;
  const nearest = starts.reduce((best, at) =>
    Math.abs(at - anchor.start) < Math.abs(best - anchor.start) ? at : best,
  );
  return { start: nearest, end: nearest + anchor.quote.length };
}

/** Whether `offset` falls inside `span`. */
export function spanContains(span: Span, offset: number): boolean {
  return offset >= span.start && offset < span.end;
}
