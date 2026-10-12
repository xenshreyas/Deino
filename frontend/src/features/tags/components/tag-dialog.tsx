/**
 * Create and edit a tag: name, colour, the description the auto-tagger
 * reads, and (when editing) whether Deino may apply it.
 */
import { type FormEvent, useState } from 'react';

import { Button } from '@/ui/button';
import { Label, Switch } from '@/ui/controls';
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/ui/dialog';
import { Input, Textarea } from '@/ui/input';

import { tagsCopy } from '../copy';
import { useCreateTag, useUpdateTag } from '../queries';
import type { PaletteColor, Tag } from '../types';
import { ColorField } from './color-field';

/** What the dialog edits: a new tag, an existing one, or nothing (closed). */
export type TagDialogTarget = { kind: 'new' } | { kind: 'edit'; tag: Tag } | null;

/** The form's fields. */
interface Draft {
  name: string;
  color: PaletteColor;
  description: string;
  autoApply: boolean;
}

function draftOf(target: Exclude<TagDialogTarget, null>): Draft {
  if (target.kind === 'new') return { name: '', color: 1, description: '', autoApply: true };
  const { name, color, description, autoApply } = target.tag;
  return { name, color, description, autoApply };
}

/** Saves the draft as a new tag or onto the edited one. */
function useSaveTag(target: Exclude<TagDialogTarget, null>, onDone: () => void) {
  const create = useCreateTag();
  const update = useUpdateTag();
  // The tag list shows the change (UX-32: no toast).
  const save = (draft: Draft) => {
    if (target.kind === 'new') {
      const tag = { name: draft.name, color: draft.color, description: draft.description };
      create.mutate(tag, { onSuccess: onDone });
    } else {
      update.mutate({ id: target.tag.id, update: draft }, { onSuccess: onDone });
    }
  };
  const pending = target.kind === 'new' ? create : update;
  return { save, error: pending.error, isPending: pending.isPending };
}

/** The fields and actions; mounted per target so it starts from its values. */
function TagForm({
  target,
  onClose,
}: {
  target: Exclude<TagDialogTarget, null>;
  onClose: () => void;
}) {
  const [draft, setDraft] = useState(() => draftOf(target));
  const { save, error, isPending } = useSaveTag(target, onClose);
  const field =
    <K extends keyof Draft>(key: K) =>
    (value: Draft[K]) => {
      setDraft((current) => ({ ...current, [key]: value }));
    };
  const submit = (event: FormEvent) => {
    event.preventDefault();
    save(draft);
  };
  return (
    <form onSubmit={submit} className="grid gap-4">
      <NameField value={draft.name} error={error?.message} onChange={field('name')} />
      <ColorField value={draft.color} onChange={field('color')} />
      <DescriptionField value={draft.description} onChange={field('description')} />
      {target.kind === 'edit' ? (
        <AutoApplyField value={draft.autoApply} onChange={field('autoApply')} />
      ) : null}
      <DialogFooter>
        <Button variant="ghost" type="button" onClick={onClose}>
          {tagsCopy.cancel}
        </Button>
        <Button type="submit" disabled={draft.name.trim() === '' || isPending}>
          {target.kind === 'new' ? tagsCopy.create : tagsCopy.save}
        </Button>
      </DialogFooter>
    </form>
  );
}

interface FieldProps<T> {
  value: T;
  onChange: (value: T) => void;
}

function NameField({ value, error, onChange }: FieldProps<string> & { error: string | undefined }) {
  return (
    <div className="grid gap-1.5">
      <Label htmlFor="tag-name">{tagsCopy.nameLabel}</Label>
      <Input
        id="tag-name"
        value={value}
        placeholder={tagsCopy.namePlaceholder}
        aria-invalid={error !== undefined}
        autoFocus
        onChange={(event) => {
          onChange(event.target.value);
        }}
      />
      {error === undefined ? null : <p className="text-xs text-danger">{error}</p>}
    </div>
  );
}

function DescriptionField({ value, onChange }: FieldProps<string>) {
  return (
    <div className="grid gap-1.5">
      <Label htmlFor="tag-description">{tagsCopy.descriptionLabel}</Label>
      <Textarea
        id="tag-description"
        rows={3}
        value={value}
        placeholder={tagsCopy.descriptionPlaceholder}
        aria-describedby="tag-description-hint"
        onChange={(event) => {
          onChange(event.target.value);
        }}
      />
      <p id="tag-description-hint" className="text-xs text-fg-muted">
        {tagsCopy.descriptionHint}
      </p>
    </div>
  );
}

function AutoApplyField({ value, onChange }: FieldProps<boolean>) {
  return (
    <div className="flex items-center justify-between gap-4">
      <Label htmlFor="tag-auto-apply">{tagsCopy.autoApplyLabel}</Label>
      <Switch id="tag-auto-apply" checked={value} onCheckedChange={onChange} />
    </div>
  );
}

/** Creates or edits a tag. */
export function TagDialog({ target, onClose }: { target: TagDialogTarget; onClose: () => void }) {
  return (
    <Dialog
      open={target !== null}
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <DialogContent aria-describedby={undefined}>
        <DialogHeader>
          <DialogTitle>
            {target?.kind === 'edit' ? tagsCopy.editTitle : tagsCopy.createTitle}
          </DialogTitle>
        </DialogHeader>
        {target === null ? null : (
          <TagForm
            key={target.kind === 'edit' ? target.tag.id : 'new'}
            target={target}
            onClose={onClose}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}
