/**
 * An item's title, edited in place: click (or Enter on focus) to edit, Enter
 * or leaving the field saves, Escape cancels. A blank title is not saved.
 */
import { type KeyboardEvent, useRef, useState } from 'react';

import { cn } from '@/ui/cn';

import { copy } from '../copy';

/** Props of {@link ItemTitle}. */
export interface ItemTitleProps {
  title: string;
  done: boolean;
  onSave: (title: string) => void;
}

/** The field while editing; saves once, on Enter or blur. */
function TitleField({ title, onDone }: { title: string; onDone: (next: string | null) => void }) {
  const [draft, setDraft] = useState(title);
  // Enter, Escape and the blur that follows them must settle only once.
  const settled = useRef(false);
  const settle = (next: string | null) => {
    if (settled.current) return;
    settled.current = true;
    onDone(next);
  };
  const finish = () => {
    const next = draft.trim();
    settle(next === '' || next === title ? null : next);
  };
  const keys = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'Enter') finish();
    if (event.key === 'Escape') settle(null);
  };
  return (
    <input
      // The user asked to edit: the field takes the focus they expect.
      autoFocus
      aria-label={copy.row.edit}
      value={draft}
      onChange={(event) => {
        setDraft(event.target.value);
      }}
      onKeyDown={keys}
      onBlur={finish}
      onClick={(event) => {
        event.stopPropagation();
      }}
      className="-ml-1 h-7 min-w-0 flex-1 rounded-sm bg-transparent px-1 text-sm text-fg ring-1 ring-border-strong outline-none focus:ring-accent"
    />
  );
}

/** The title, or its field while editing. */
export function ItemTitle({ title, done, onSave }: ItemTitleProps) {
  const [editing, setEditing] = useState(false);
  if (editing) {
    return (
      <TitleField
        title={title}
        onDone={(next) => {
          setEditing(false);
          if (next !== null) onSave(next);
        }}
      />
    );
  }
  return (
    <button
      type="button"
      title={copy.row.edit}
      onClick={(event) => {
        event.stopPropagation();
        setEditing(true);
      }}
      className={cn(
        'min-w-0 cursor-text truncate rounded-sm text-left text-sm transition-colors duration-(--dur-base) ease-out',
        'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus-ring',
        done ? 'text-fg-subtle line-through decoration-fg-subtle' : 'text-fg',
      )}
    >
      {title}
    </button>
  );
}
