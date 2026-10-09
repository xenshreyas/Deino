/**
 * Names a folder: creates one (at the top level or inside a parent) or
 * renames one. Errors from Rust (a blank or taken name) show under the field.
 */
import { type FormEvent, useState } from 'react';
import { toast } from 'sonner';

import { Button } from '@/ui/button';
import { Label } from '@/ui/controls';
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/ui/dialog';
import { Input } from '@/ui/input';

import { foldersCopy } from '../copy';
import { useCreateFolder, useRenameFolder } from '../mutations';
import type { Folder } from '../types';

/** What the dialog names, or `null` while closed. */
export type FolderNameTarget =
  { kind: 'create'; parent: Folder | null } | { kind: 'rename'; folder: Folder } | null;

/** Props of {@link FolderNameDialog}. */
export interface FolderNameDialogProps {
  target: FolderNameTarget;
  onClose: () => void;
  /** Called with a newly created folder (to file a meeting into it, say). */
  onCreated?: ((folder: Folder) => void) | undefined;
}

function titleOf(target: Exclude<FolderNameTarget, null>): string {
  if (target.kind === 'rename') return foldersCopy.renameTitle;
  return target.parent === null
    ? foldersCopy.createTitle
    : foldersCopy.createInTitle(target.parent.name);
}

/** Creates or renames, then closes. */
function useSaveName(target: Exclude<FolderNameTarget, null>, props: FolderNameDialogProps) {
  const create = useCreateFolder();
  const rename = useRenameFolder();
  const pending = target.kind === 'create' ? create : rename;
  const save = (name: string) => {
    if (target.kind === 'rename') {
      rename.mutate({ id: target.folder.id, name }, { onSuccess: props.onClose });
      return;
    }
    const folder = { name, parentId: target.parent?.id ?? null };
    create.mutate(folder, {
      onSuccess: (created) => {
        toast.success(foldersCopy.created);
        props.onCreated?.(created);
        props.onClose();
      },
    });
  };
  return { save, error: pending.error, isPending: pending.isPending };
}

/** The field and actions; mounted per target so it starts from its name. */
function NameForm({
  target,
  ...props
}: FolderNameDialogProps & { target: Exclude<FolderNameTarget, null> }) {
  const [name, setName] = useState(target.kind === 'rename' ? target.folder.name : '');
  const { save, error, isPending } = useSaveName(target, { target, ...props });
  const submit = (event: FormEvent) => {
    event.preventDefault();
    save(name);
  };
  return (
    <form onSubmit={submit} className="grid gap-4">
      <div className="grid gap-1.5">
        <Label htmlFor="folder-name">{foldersCopy.nameLabel}</Label>
        <Input
          id="folder-name"
          value={name}
          placeholder={foldersCopy.namePlaceholder}
          aria-invalid={error !== null}
          autoFocus
          onChange={(event) => {
            setName(event.target.value);
          }}
        />
        {error === null ? null : <p className="text-xs text-danger">{error.message}</p>}
      </div>
      <DialogFooter>
        <Button variant="ghost" type="button" onClick={props.onClose}>
          {foldersCopy.cancel}
        </Button>
        <Button type="submit" disabled={name.trim() === '' || isPending}>
          {target.kind === 'create' ? foldersCopy.create : foldersCopy.save}
        </Button>
      </DialogFooter>
    </form>
  );
}

/** Creates or renames a folder. */
export function FolderNameDialog(props: FolderNameDialogProps) {
  const { target, onClose } = props;
  return (
    <Dialog
      open={target !== null}
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <DialogContent aria-describedby={undefined}>
        <DialogHeader>
          <DialogTitle>{target === null ? foldersCopy.createTitle : titleOf(target)}</DialogTitle>
        </DialogHeader>
        {target === null ? null : (
          <NameForm
            key={target.kind === 'rename' ? target.folder.id : 'create'}
            {...props}
            target={target}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}
