/**
 * A snippet with its matches marked in `--highlight` (ui.md §3 Command
 * palette: "a matched snippet with `--highlight` on the hit").
 */
import type { Snippet } from '../types';

/** One run of a snippet: plain or matched text. */
export interface SnippetPart {
  text: string;
  matched: boolean;
  /** Where the part starts in the snippet: a stable key. */
  at: number;
}

/**
 * `snippet` cut at its highlight ranges. Ranges are UTF-16 indices, so plain
 * `slice` cuts exactly where Rust meant; out-of-range or overlapping ranges
 * are clamped instead of trusted.
 */
export function snippetParts(snippet: Snippet): SnippetPart[] {
  const parts: SnippetPart[] = [];
  let cursor = 0;
  for (const { start, end } of snippet.highlights) {
    const from = Math.max(start, cursor);
    const to = Math.min(end, snippet.text.length);
    if (to <= from) continue;
    if (from > cursor)
      parts.push({ text: snippet.text.slice(cursor, from), matched: false, at: cursor });
    parts.push({ text: snippet.text.slice(from, to), matched: true, at: from });
    cursor = to;
  }
  if (cursor < snippet.text.length) {
    parts.push({ text: snippet.text.slice(cursor), matched: false, at: cursor });
  }
  return parts;
}

/** Props of {@link HighlightedText}. */
export interface HighlightedTextProps {
  snippet: Snippet;
  className?: string;
}

/** The snippet's text, its matches in `<mark>`. */
export function HighlightedText({ snippet, className }: HighlightedTextProps) {
  return (
    <span className={className}>
      {snippetParts(snippet).map((part) =>
        part.matched ? (
          <mark key={part.at} className="rounded-[3px] bg-highlight px-px text-fg">
            {part.text}
          </mark>
        ) : (
          <span key={part.at}>{part.text}</span>
        ),
      )}
    </span>
  );
}
