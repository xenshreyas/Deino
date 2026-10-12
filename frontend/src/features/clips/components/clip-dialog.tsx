/**
 * "New clip": the range chosen in the transcript, a name (the selected
 * words to start with), and Create. Cutting takes a moment; the dialog
 * stays until the clip is saved, then closes.
 */
import { type FormEvent, useState } from 'react';

import { Button } from '@/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/ui/dialog';
import { Input } from '@/ui/input';

import { clipsCopy } from '../copy';
import { useCreateClip } from '../queries';
import { cancelClip, useClipDraftStore } from '../store';
import type { ClipDraft } from '../types';
import { clipRange } from '../time';

/** The longest name the dialog suggests from the selection. */
const SUGGESTED_NAME_LENGTH = 60;

/** The selected words, shortened to a name. */
export function suggestName(text: string): string {
  const words = text.trim().replace(/\s+/g, ' ');
  if (words.length <= SUGGESTED_NAME_LENGTH) return words;
  return `${words.slice(0, SUGGESTED_NAME_LENGTH).trimEnd()}…`;
}

/** The form for one draft; remounted per draft so its name starts fresh. */
function ClipForm({ draft }: { draft: ClipDraft }) {
  const [name, setName] = useState(suggestName(draft.suggestedName));
  const create = useCreateClip();
  const submit = (event: FormEvent) => {
    event.preventDefault();
    const clip = { meetingId: draft.meetingId, name, start: draft.start, end: draft.end };
    create.mutate(clip, { onSuccess: cancelClip });
  };
  const range = clipRange(draft.start, draft.end);
  return (
    <form onSubmit={submit} className="flex flex-col gap-4">
      <DialogHeader>
        <DialogTitle>{clipsCopy.dialogTitle}</DialogTitle>
        <DialogDescription>{clipsCopy.dialogDescription(range)}</DialogDescription>
      </DialogHeader>
      <label className="flex flex-col gap-1.5 text-sm text-fg-muted">
        {clipsCopy.nameLabel}
        <Input
          autoFocus
          value={name}
          placeholder={clipsCopy.namePlaceholder}
          onChange={(event) => {
            setName(event.target.value);
          }}
        />
      </label>
      <DialogFooter>
        <Button type="button" variant="ghost" onClick={cancelClip}>
          {clipsCopy.cancel}
        </Button>
        <Button type="submit" disabled={name.trim() === '' || create.isPending}>
          {create.isPending ? clipsCopy.creating : clipsCopy.create}
        </Button>
      </DialogFooter>
    </form>
  );
}

/** The dialog, open while a clip is being named. */
export function ClipDialog() {
  const draft = useClipDraftStore((state) => state.draft);
  return (
    <Dialog
      open={draft !== null}
      onOpenChange={(open) => {
        if (!open) cancelClip();
      }}
    >
      <DialogContent>
        {draft === null ? null : (
          <ClipForm key={`${String(draft.start)}-${String(draft.end)}`} draft={draft} />
        )}
      </DialogContent>
    </Dialog>
  );
}
