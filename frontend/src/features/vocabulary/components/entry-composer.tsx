/**
 * The two-field row that adds or edits a vocabulary entry: a term's
 * spelling and what it sounds like, or a replacement's phrases and what
 * they become. Enter saves; Escape leaves an edit.
 */
import { type FormEvent, type KeyboardEvent, useState } from 'react';
import { toast } from 'sonner';

import { Button } from '@/ui/button';
import { cn } from '@/ui/cn';
import { ArrowRight, ICON_PROPS } from '@/ui/icons';
import { Input } from '@/ui/input';

import { vocabularyCopy as copy } from '../copy';
import { termInput } from '../entries';
import { useCreateTerm, useUpdateTerm } from '../queries';
import type { VocabKind, VocabTerm } from '../types';

/** Props of {@link EntryComposer}. */
export interface EntryComposerProps {
  kind: VocabKind;
  /** The entry being edited; absent to add a new one. */
  editing?: VocabTerm | undefined;
  /** Leaves the edit (after saving, or on Cancel and Escape). */
  onDone?: (() => void) | undefined;
}

/** Saves the fields as a new entry, or over `editing`. */
function useSave(kind: VocabKind, editing: VocabTerm | undefined) {
  const create = useCreateTerm();
  const update = useUpdateTerm();
  const save = (spelled: string, soundsLike: string, onSaved: () => void) => {
    const input = termInput(kind, spelled, soundsLike);
    const onError = (error: Error) => {
      toast.error(copy.saveFailed, { description: error.message });
    };
    if (editing === undefined) create.mutate(input, { onSuccess: onSaved, onError });
    else update.mutate({ id: editing.id, input }, { onSuccess: onSaved, onError });
  };
  return { save, pending: create.isPending || update.isPending };
}

/** Field labels and placeholders for each kind: `spelled` is what it becomes. */
const FIELDS = {
  term: {
    spelled: [copy.termLabel, copy.termPlaceholder],
    soundsLike: [copy.soundsLikeLabel, copy.soundsLikePlaceholder],
  },
  replacement: {
    spelled: [copy.withLabel, copy.withPlaceholder],
    soundsLike: [copy.replaceLabel, copy.replacePlaceholder],
  },
} as const;

/** One field with its visible label above it. */
function Field(props: {
  id: string;
  label: string;
  placeholder: string;
  value: string;
  onChange: (value: string) => void;
  className: string;
}) {
  return (
    <label htmlFor={props.id} className={cn('flex flex-col gap-1', props.className)}>
      <span className="text-xs text-fg-muted">{props.label}</span>
      <Input
        id={props.id}
        value={props.value}
        placeholder={props.placeholder}
        autoComplete="off"
        spellCheck={false}
        onChange={(event) => {
          props.onChange(event.target.value);
        }}
      />
    </label>
  );
}

/** Adds or edits one entry. */
export function EntryComposer({ kind, editing, onDone }: EntryComposerProps) {
  const [spelled, setSpelled] = useState(editing?.text ?? '');
  const [soundsLike, setSoundsLike] = useState(editing?.soundsLike.join(', ') ?? '');
  const { save, pending } = useSave(kind, editing);
  const id = `vocabulary-${kind}-${editing?.id ?? 'new'}`;
  const submit = (event: FormEvent) => {
    event.preventDefault();
    save(spelled, soundsLike, () => {
      setSpelled('');
      setSoundsLike('');
      onDone?.();
    });
  };
  const leave = (event: KeyboardEvent) => {
    if (event.key === 'Escape' && onDone !== undefined) onDone();
  };
  const ready = spelled.trim().length > 0 && (kind === 'term' || soundsLike.trim().length > 0);
  return (
    <form onSubmit={submit} onKeyDown={leave} className="flex items-end gap-2 px-4 py-3">
      <ComposerFields
        kind={kind}
        id={id}
        spelled={[spelled, setSpelled]}
        soundsLike={[soundsLike, setSoundsLike]}
      />
      <ComposerActions
        editing={editing !== undefined}
        ready={ready && !pending}
        onCancel={onDone}
      />
    </form>
  );
}

/** The two fields, in reading order for the kind. */
function ComposerFields(props: {
  kind: VocabKind;
  id: string;
  spelled: [string, (value: string) => void];
  soundsLike: [string, (value: string) => void];
}) {
  const fields = FIELDS[props.kind];
  const spelled = (
    <Field
      key="spelled"
      id={`${props.id}-spelled`}
      label={fields.spelled[0]}
      placeholder={fields.spelled[1]}
      value={props.spelled[0]}
      onChange={props.spelled[1]}
      className="w-40 shrink-0"
    />
  );
  const soundsLike = (
    <Field
      key="sounds-like"
      id={`${props.id}-sounds-like`}
      label={fields.soundsLike[0]}
      placeholder={fields.soundsLike[1]}
      value={props.soundsLike[0]}
      onChange={props.soundsLike[1]}
      className="min-w-0 flex-1"
    />
  );
  if (props.kind === 'term') return [spelled, soundsLike];
  return [
    soundsLike,
    <ArrowRight key="arrow" {...ICON_PROPS} className="mb-2 shrink-0 text-fg-subtle" />,
    spelled,
  ];
}

/** Save (or Add), and Cancel while editing. */
function ComposerActions(props: {
  editing: boolean;
  ready: boolean;
  onCancel: (() => void) | undefined;
}) {
  return (
    <div className="flex shrink-0 gap-1">
      {props.editing ? (
        <Button type="button" variant="ghost" onClick={props.onCancel}>
          {copy.cancel}
        </Button>
      ) : null}
      <Button type="submit" variant="secondary" disabled={!props.ready}>
        {props.editing ? copy.save : copy.add}
      </Button>
    </div>
  );
}
