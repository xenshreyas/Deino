/**
 * Import recipes: paste a recipes file or a `deino://recipe` share string,
 * or read a file into the field. Imported recipes become the user's own.
 */
import { type ChangeEvent, useState } from 'react';
import { toast } from 'sonner';

import { Button } from '@/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/ui/dialog';
import { Textarea } from '@/ui/input';

import { recipesCopy } from '../copy';
import { useImportRecipes } from '../queries';
import type { Recipe } from '../types';

/** Props of {@link ImportDialog}. */
export interface ImportDialogProps {
  open: boolean;
  onClose: () => void;
  onImported: (recipes: Recipe[]) => void;
}

/** The dialog; its form remounts per opening. */
export function ImportDialog({ open, onClose, onImported }: ImportDialogProps) {
  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) onClose();
      }}
    >
      <DialogContent className="max-w-[560px]">
        <DialogHeader>
          <DialogTitle>{recipesCopy.importTitle}</DialogTitle>
          <DialogDescription>{recipesCopy.importBody}</DialogDescription>
        </DialogHeader>
        {open ? <ImportForm onClose={onClose} onImported={onImported} /> : null}
      </DialogContent>
    </Dialog>
  );
}

function ImportForm({ onClose, onImported }: Omit<ImportDialogProps, 'open'>) {
  const [text, setText] = useState('');
  const importer = useImportRecipes();
  const choose = (event: ChangeEvent<HTMLInputElement>) => {
    void event.target.files?.[0]?.text().then(setText);
  };
  const submit = () => {
    importer.mutate(text, {
      onSuccess: (recipes) => {
        toast.success(recipesCopy.imported(recipes.length));
        onImported(recipes);
      },
    });
  };
  return (
    <>
      <div className="grid gap-2">
        <FileField onChange={choose} />
        <Textarea
          aria-label={recipesCopy.importField}
          value={text}
          rows={8}
          className="font-mono text-xs"
          onChange={(event) => {
            setText(event.target.value);
          }}
        />
        {importer.isError ? <ImportError message={importer.error.message} /> : null}
      </div>
      <DialogFooter>
        <Button variant="secondary" onClick={onClose}>
          {recipesCopy.cancel}
        </Button>
        <Button disabled={text.trim() === '' || importer.isPending} onClick={submit}>
          {recipesCopy.import}
        </Button>
      </DialogFooter>
    </>
  );
}

/** Reads a recipes file into the field. */
function FileField({ onChange }: { onChange: (event: ChangeEvent<HTMLInputElement>) => void }) {
  return (
    <input
      type="file"
      accept=".json,application/json"
      aria-label={recipesCopy.importFile}
      onChange={onChange}
      className="text-sm text-fg-muted file:mr-3 file:rounded-md file:border file:border-control-border file:bg-control file:px-3 file:py-1 file:text-sm file:text-fg"
    />
  );
}

function ImportError({ message }: { message: string }) {
  return (
    <p role="alert" className="text-xs text-danger">
      {message}
    </p>
  );
}
