/**
 * The insight editor: name, what to extract, typed fields, how many answers
 * per meeting, and when it runs. Opens on a template, a blank insight or an
 * existing one; Rust validates on save and the error shows inline.
 */
import { type FormEvent, type ReactNode, useState } from 'react';

import { Button } from '@/ui/button';
import { Label } from '@/ui/controls';
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/ui/dialog';
import { CircleAlert, ICON_PROPS } from '@/ui/icons';
import { Input, Textarea } from '@/ui/input';
import { SegmentedControl } from '@/ui/segmented-control';

import { editorCopy } from '../copy';
import { useSaveInsight } from '../queries';
import type { InsightDef, InsightId, InsightInput } from '../types';
import { ConditionGroups } from './condition-groups';
import { type Draft, toDraft, toInput, type WhenMode } from './draft';
import { FieldRows } from './field-rows';

/** What the editor edits; `null` when closed. */
export interface EditorTarget {
  /** The insight to replace, or `null` to create one. */
  id: InsightId | null;
  input: InsightInput;
}

/** Props of {@link InsightEditor}. */
export interface InsightEditorProps {
  target: EditorTarget | null;
  onClose: () => void;
  onSaved: (def: InsightDef) => void;
}

/** The editor, in a dialog; the form remounts for each target. */
export function InsightEditor({ target, onClose, onSaved }: InsightEditorProps) {
  return (
    <Dialog
      open={target !== null}
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <DialogContent className="max-w-[560px]">
        <DialogHeader>
          <DialogTitle>
            {target !== null && target.id !== null ? editorCopy.editTitle : editorCopy.createTitle}
          </DialogTitle>
        </DialogHeader>
        {target === null ? null : (
          <EditorForm target={target} onClose={onClose} onSaved={onSaved} />
        )}
      </DialogContent>
    </Dialog>
  );
}

/** A setter for one key of the draft. */
type SetDraft = <K extends keyof Draft>(key: K) => (value: Draft[K]) => void;

/** The form over its draft. */
function EditorForm({ target, onClose, onSaved }: InsightEditorProps & { target: EditorTarget }) {
  const [draft, setDraft] = useState<Draft>(() => toDraft(target.input));
  const save = useSaveInsight();
  const set: SetDraft = (key) => (value) => {
    setDraft((current) => ({ ...current, [key]: value }));
  };
  const submit = (event: FormEvent) => {
    event.preventDefault();
    save.mutate({ id: target.id, input: toInput(draft) }, { onSuccess: onSaved });
  };
  return (
    <form onSubmit={submit} className="grid gap-5">
      <TextFields draft={draft} set={set} />
      <FieldRows fields={draft.fields} onChange={set('fields')} />
      <CountField draft={draft} onCount={set('count')} onMode={set('countMode')} />
      <WhenField draft={draft} onWhen={set('when')} onGroups={set('groups')} />
      {save.error === null ? null : <ErrorLine message={save.error.message} />}
      <DialogFooter>
        <Button type="button" variant="ghost" onClick={onClose}>
          {editorCopy.cancel}
        </Button>
        <Button type="submit" disabled={save.isPending}>
          {target.id === null ? editorCopy.create : editorCopy.save}
        </Button>
      </DialogFooter>
    </form>
  );
}

/** The name and the description (the prompt). */
function TextFields({ draft, set }: { draft: Draft; set: SetDraft }) {
  return (
    <>
      <Field label={editorCopy.name} id="insight-name">
        <Input
          id="insight-name"
          value={draft.name}
          placeholder={editorCopy.namePlaceholder}
          maxLength={80}
          required
          onChange={(event) => {
            set('name')(event.target.value);
          }}
        />
      </Field>
      <Field
        label={editorCopy.description}
        id="insight-description"
        hint={editorCopy.descriptionHint}
      >
        <Textarea
          id="insight-description"
          rows={4}
          value={draft.description}
          required
          onChange={(event) => {
            set('description')(event.target.value);
          }}
        />
      </Field>
    </>
  );
}

/** A labelled control with an optional hint. */
function Field({
  label,
  id,
  hint,
  children,
}: {
  label: string;
  id: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <div className="grid gap-1.5">
      <Label htmlFor={id}>{label}</Label>
      {children}
      {hint === undefined ? null : <p className="text-xs text-fg-muted">{hint}</p>}
    </div>
  );
}

/** Auto, or at most N answers. */
function CountField({
  draft,
  onCount,
  onMode,
}: {
  draft: Draft;
  onCount: (count: number) => void;
  onMode: (mode: Draft['countMode']) => void;
}) {
  return (
    <div className="grid gap-1.5">
      <span className="text-sm font-semibold text-fg">{editorCopy.count}</span>
      <div className="flex items-center gap-2">
        <SegmentedControl
          label={editorCopy.count}
          value={draft.countMode}
          onValueChange={onMode}
          segments={[
            { value: 'auto', label: editorCopy.countAuto },
            { value: 'fixed', label: editorCopy.countFixed },
          ]}
        />
        {draft.countMode === 'fixed' ? (
          <Input
            type="number"
            aria-label={editorCopy.countFixed}
            min={1}
            max={20}
            value={draft.count}
            className="w-16"
            onChange={(event) => {
              onCount(Math.round(Number(event.target.value)) || 1);
            }}
          />
        ) : null}
      </div>
    </div>
  );
}

const WHEN_SEGMENTS: { value: WhenMode; label: string }[] = [
  { value: 'always', label: editorCopy.whenAutomatic },
  { value: 'matching', label: editorCopy.whenMatching },
  { value: 'manual', label: editorCopy.whenManual },
];

/** Every meeting, matching meetings (with their conditions), or by hand. */
function WhenField({
  draft,
  onWhen,
  onGroups,
}: {
  draft: Draft;
  onWhen: (when: WhenMode) => void;
  onGroups: (groups: Draft['groups']) => void;
}) {
  return (
    <div className="grid gap-2">
      <span className="text-sm font-semibold text-fg">{editorCopy.when}</span>
      <SegmentedControl
        label={editorCopy.when}
        value={draft.when}
        onValueChange={onWhen}
        segments={WHEN_SEGMENTS}
        className="self-start"
      />
      {draft.when === 'matching' ? (
        <ConditionGroups groups={draft.groups} onChange={onGroups} />
      ) : null}
    </div>
  );
}

/** A save error, in words the user can act on. */
function ErrorLine({ message }: { message: string }) {
  return (
    <p role="alert" className="flex items-start gap-1.5 text-xs text-danger">
      <CircleAlert {...ICON_PROPS} size={14} className="mt-px shrink-0" />
      {message}
    </p>
  );
}
