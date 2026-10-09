import { type FormEvent, useState } from 'react';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { ICON_PROPS, Plus } from '@/ui/icons';

import { copy } from '../copy';

interface AddNoteFormProps {
  /** True while a note is being saved. */
  saving: boolean;
  /** A user-safe message from the last failed save, if any. */
  error: string | undefined;
  /** Asks to save `text`; `onSaved` runs once it is stored, so the form clears. */
  onAdd: (text: string, onSaved: () => void) => void;
}

/** A one-line form that adds a note. Presentational: data in, intents out. */
export function AddNoteForm({ saving, error, onAdd }: AddNoteFormProps) {
  const [text, setText] = useState('');

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    // On failure the text stays, so the user can fix it next to the error.
    onAdd(text, () => {
      setText('');
    });
  };

  return (
    <form onSubmit={submit} className="flex flex-col gap-2">
      <div className="flex gap-2">
        <Input
          value={text}
          onChange={(event) => {
            setText(event.target.value);
          }}
          placeholder={copy.placeholder}
          aria-label={copy.placeholder}
          aria-invalid={error !== undefined}
        />
        <Button type="submit" disabled={saving}>
          <Plus {...ICON_PROPS} />
          {copy.add}
        </Button>
      </div>
      {error !== undefined && (
        <p role="alert" className="text-sm text-destructive">
          {error}
        </p>
      )}
    </form>
  );
}
