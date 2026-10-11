/**
 * The user's own notes about a person or company: a borderless field that
 * grows with its text and saves itself shortly after typing stops, and on
 * leaving it. Mounted per entity (keyed), so it starts from the stored text.
 */
import { useRef, useState } from 'react';
import { toast } from 'sonner';

import { entitiesCopy } from '../copy';
import { Section } from './entity-sections';

/** How long typing pauses before the notes save, in ms. */
const SAVE_DELAY_MS = 800;

/** Props of {@link EntityNotes}. */
export interface EntityNotesProps {
  initial: string;
  save: (notes: string) => Promise<unknown>;
}

/** Saves the latest text once typing settles; `flush` saves now. */
function useAutosave(initial: string, save: (notes: string) => Promise<unknown>) {
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const saved = useRef(initial);
  const [status, setStatus] = useState<'idle' | 'saved'>('idle');
  const flush = (text: string) => {
    if (timer.current !== null) clearTimeout(timer.current);
    timer.current = null;
    if (text === saved.current) return;
    saved.current = text;
    save(text).then(
      () => {
        setStatus('saved');
      },
      (error: unknown) => {
        const description = error instanceof Error ? error.message : undefined;
        toast.error(entitiesCopy.notesFailed, { description });
      },
    );
  };
  const schedule = (text: string) => {
    setStatus('idle');
    if (timer.current !== null) clearTimeout(timer.current);
    timer.current = setTimeout(() => {
      flush(text);
    }, SAVE_DELAY_MS);
  };
  return { status, flush, schedule };
}

/** The notes block. */
export function EntityNotes({ initial, save }: EntityNotesProps) {
  const [text, setText] = useState(initial);
  const { status, flush, schedule } = useAutosave(initial, save);
  return (
    <Section
      id="entity-notes"
      title={entitiesCopy.notes}
      trailing={
        status === 'saved' ? (
          <span className="text-xs text-fg-subtle">{entitiesCopy.notesSaved}</span>
        ) : null
      }
    >
      <textarea
        aria-labelledby="entity-notes"
        value={text}
        rows={3}
        placeholder={entitiesCopy.notesPlaceholder}
        onChange={(event) => {
          setText(event.target.value);
          schedule(event.target.value);
        }}
        onBlur={() => {
          flush(text);
        }}
        className="field-sizing-content min-h-20 w-full resize-none rounded-md bg-transparent px-3 py-2 text-sm leading-6 text-user-text outline-none placeholder:text-fg-subtle hover:bg-bg-subtle focus-visible:bg-bg-subtle"
      />
    </Section>
  );
}
