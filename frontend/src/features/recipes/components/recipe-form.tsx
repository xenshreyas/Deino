/**
 * The editor of one of the user's recipes (or a new one): name (its `/slug`
 * follows), a one-line description and the prompt. Remount with `key` to
 * start from another recipe.
 */
import { type ReactNode, useState } from 'react';

import { Button } from '@/ui/button';
import { Label } from '@/ui/controls';
import { Input, Textarea } from '@/ui/input';

import { recipesCopy } from '../copy';
import type { RecipeDraft } from '../types';

/** Props of {@link RecipeForm}. */
export interface RecipeFormProps {
  initial: RecipeDraft;
  saving: boolean;
  onSave: (draft: RecipeDraft) => void;
  /** Buttons beside Save. */
  actions?: ReactNode;
}

/** The three fields and Save. */
export function RecipeForm({ initial, saving, onSave, actions }: RecipeFormProps) {
  const [draft, setDraft] = useState<RecipeDraft>(initial);
  const set = (field: keyof RecipeDraft) => (value: string) => {
    setDraft((current) => ({ ...current, [field]: value }));
  };
  const dirty =
    draft.name !== initial.name ||
    draft.description !== initial.description ||
    draft.prompt !== initial.prompt;
  const valid = draft.name.trim() !== '' && draft.prompt.trim() !== '';
  return (
    <form
      className="grid gap-4"
      onSubmit={(event) => {
        event.preventDefault();
        onSave(draft);
      }}
    >
      <Fields draft={draft} set={set} />
      <div className="flex items-center gap-2">
        <Button type="submit" disabled={!dirty || !valid || saving}>
          {recipesCopy.save}
        </Button>
        {actions}
      </div>
    </form>
  );
}

/** The one-line fields, in order. */
const LINE_FIELDS = [
  ['name', recipesCopy.name],
  ['description', recipesCopy.descriptionField],
] as const;

/** The three fields. */
function Fields({
  draft,
  set,
}: {
  draft: RecipeDraft;
  set: (field: keyof RecipeDraft) => (value: string) => void;
}) {
  return (
    <>
      {LINE_FIELDS.map(([field, label]) => (
        <Field key={field} id={`recipe-${field}`} label={label}>
          <Input
            id={`recipe-${field}`}
            value={draft[field]}
            onChange={(e) => {
              set(field)(e.target.value);
            }}
          />
        </Field>
      ))}
      <Field id="recipe-prompt" label={recipesCopy.prompt}>
        <Textarea
          id="recipe-prompt"
          rows={8}
          value={draft.prompt}
          placeholder={recipesCopy.newPrompt}
          onChange={(e) => {
            set('prompt')(e.target.value);
          }}
        />
      </Field>
    </>
  );
}

function Field({ id, label, children }: { id: string; label: string; children: ReactNode }) {
  return (
    <div className="grid gap-1.5">
      <Label htmlFor={id}>{label}</Label>
      {children}
    </div>
  );
}
