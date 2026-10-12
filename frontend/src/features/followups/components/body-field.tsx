/**
 * The draft's message field: a plain textarea whose `[placeholders]` are
 * highlighted. A mirror of the text sits behind the transparent textarea in
 * the same grid cell, with the same font, padding and wrapping; only its
 * highlights show. The mirror also sizes the cell, so the field grows with
 * the text and never scrolls on its own (no scroll syncing).
 */
import { Fragment } from 'react';

import { cn } from '@/ui/cn';

import { textRuns } from '../draft-text';

/** What the mirror and the textarea share, so their text lines up exactly. */
const SHARED =
  'col-start-1 row-start-1 min-h-48 w-full px-3 py-2.5 text-sm leading-6 whitespace-pre-wrap break-words font-[inherit]';

/** Props of {@link BodyField}. */
export interface BodyFieldProps {
  id: string;
  /** The field's accessible name. */
  label: string;
  value: string;
  onChange: (value: string) => void;
}

/** The highlights behind the text. */
function Mirror({ value }: { value: string }) {
  return (
    <div aria-hidden className={cn(SHARED, 'pointer-events-none text-transparent')}>
      {textRuns(value).map((run, index) =>
        run.placeholder ? (
          <mark key={index} className="rounded-sm bg-highlight text-transparent">
            {run.text}
          </mark>
        ) : (
          <Fragment key={index}>{run.text}</Fragment>
        ),
      )}
      {/* A trailing line break keeps a final empty line's height. */}
      {'\n'}
    </div>
  );
}

/** The message, with its gaps highlighted. */
export function BodyField({ id, label, value, onChange }: BodyFieldProps) {
  return (
    <div
      className={cn(
        'max-h-[45vh] overflow-y-auto rounded-md border border-border bg-surface',
        'transition-[border-color,box-shadow] duration-(--dur-fast) ease-out',
        'focus-within:border-accent hover:border-border-strong',
        'focus-within:shadow-[0_0_0_3px_color-mix(in_srgb,var(--accent)_20%,transparent)]',
      )}
    >
      {/* The field scrolls as a whole, so the mirror and the text move together. */}
      <div className="grid">
        <Mirror value={value} />
        <textarea
          id={id}
          aria-label={label}
          value={value}
          spellCheck
          className={cn(SHARED, 'resize-none overflow-hidden bg-transparent text-fg outline-none')}
          onChange={(event) => {
            onChange(event.target.value);
          }}
        />
      </div>
    </div>
  );
}
