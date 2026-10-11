/**
 * The table of contents beside notes with three or more headings (ui.md
 * §4.2, Circleback §5): 12/400 `fg-muted`, the section in view in `fg`.
 * It floats at the right of the note column when there is room and the
 * right panel is closed.
 */
import type { NoteDoc } from '@/features/notepad';
import { cn } from '@/ui/cn';

import { notesCopy } from '../copy';
import type { TocEntry } from '../types';

/** Fewer headings than this need no contents. */
export const TOC_MIN_HEADINGS = 3;

/** The document's headings, in order. */
export function tocEntries(doc: NoteDoc): TocEntry[] {
  const entries: TocEntry[] = [];
  for (const block of doc.content) {
    if (block.type !== 'heading') continue;
    const text = (block.content ?? [])
      .map((inline) => (inline.type === 'text' ? inline.text : ''))
      .join('')
      .trim();
    if (text !== '') entries.push({ index: entries.length, text, level: block.attrs.level });
  }
  return entries;
}

/** Props of {@link NotesToc}. */
export interface NotesTocProps {
  entries: TocEntry[];
  /** The heading in view. */
  active: number;
  onJump: (entry: TocEntry) => void;
}

/** The contents list; nothing below {@link TOC_MIN_HEADINGS}. */
export function NotesToc({ entries, active, onJump }: NotesTocProps) {
  if (entries.length < TOC_MIN_HEADINGS) return null;
  return (
    <nav
      aria-label={notesCopy.contents}
      className="absolute top-12 left-full ml-10 hidden w-44 min-[1400px]:block"
    >
      <p className="mb-2 text-xs font-semibold text-fg-subtle">{notesCopy.contents}</p>
      <ol className="flex flex-col gap-0.5 border-l border-border">
        {entries.map((entry) => (
          <li key={entry.index}>
            <button
              type="button"
              aria-current={entry.index === active ? 'location' : undefined}
              onClick={() => {
                onJump(entry);
              }}
              className={cn(
                '-ml-px w-full truncate border-l py-1 pl-3 text-left text-xs transition-colors duration-(--dur-fast)',
                'focus-visible:outline-2 focus-visible:outline-focus-ring',
                entry.index === active
                  ? 'border-fg text-fg'
                  : 'border-transparent text-fg-muted hover:text-fg',
              )}
            >
              {entry.text}
            </button>
          </li>
        ))}
      </ol>
    </nav>
  );
}
