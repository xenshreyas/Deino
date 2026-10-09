import { type ChangeEvent, useState } from 'react';

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

import { templatesCopy } from '../copy';
import { useDeleteTemplate, useImportTemplate } from '../queries';
import type { Template } from '../types';

/** Confirms deleting one of the user's templates; `template` is `null` while closed. */
export function DeleteTemplateDialog({
  template,
  onClose,
}: {
  template: Template | null;
  onClose: () => void;
}) {
  const remove = useDeleteTemplate();
  return (
    <Dialog
      open={template !== null}
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{templatesCopy.deleteTitle}</DialogTitle>
          <DialogDescription>{templatesCopy.deleteBody}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="ghost" onClick={onClose}>
            {templatesCopy.cancel}
          </Button>
          <Button
            variant="danger-solid"
            disabled={remove.isPending}
            onClick={() => {
              if (template !== null) remove.mutate(template.id, { onSuccess: onClose });
            }}
          >
            {templatesCopy.delete}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

/** Props of {@link ImportTemplateDialog}. */
export interface ImportTemplateDialogProps {
  open: boolean;
  onClose: () => void;
  onImported: (template: Template) => void;
}

/** Imports a template file: paste its JSON or choose the file. */
export function ImportTemplateDialog({ open, onClose, onImported }: ImportTemplateDialogProps) {
  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) onClose();
      }}
    >
      <DialogContent className="max-w-[560px]">
        <DialogHeader>
          <DialogTitle>{templatesCopy.importTitle}</DialogTitle>
          <DialogDescription>{templatesCopy.importBody}</DialogDescription>
        </DialogHeader>
        {open ? <ImportForm onClose={onClose} onImported={onImported} /> : null}
      </DialogContent>
    </Dialog>
  );
}

/** The file picker, the JSON field and the buttons; remounted per opening. */
function ImportForm({ onClose, onImported }: Omit<ImportTemplateDialogProps, 'open'>) {
  const [json, setJson] = useState('');
  const importer = useImportTemplate();
  const choose = (event: ChangeEvent<HTMLInputElement>) => {
    void event.target.files?.[0]?.text().then(setJson);
  };
  return (
    <>
      <div className="grid gap-2">
        <FileField onChange={choose} />
        <Textarea
          aria-label={templatesCopy.importJson}
          value={json}
          rows={8}
          className="font-mono text-xs"
          onChange={(event) => {
            setJson(event.target.value);
          }}
        />
        {importer.isError ? (
          <p role="alert" className="text-xs text-danger">
            {importer.error.message}
          </p>
        ) : null}
      </div>
      <ImportFooter
        disabled={json.trim() === '' || importer.isPending}
        onCancel={onClose}
        onImport={() => {
          importer.mutate(json, { onSuccess: onImported });
        }}
      />
    </>
  );
}

/** Chooses a template file to read into the JSON field. */
function FileField({ onChange }: { onChange: (event: ChangeEvent<HTMLInputElement>) => void }) {
  return (
    <input
      type="file"
      accept=".json,application/json"
      aria-label={templatesCopy.importFile}
      onChange={onChange}
      className="text-sm text-fg-muted file:mr-3 file:rounded-md file:border file:border-control-border file:bg-control file:px-3 file:py-1 file:text-sm file:text-fg"
    />
  );
}

function ImportFooter(props: { disabled: boolean; onCancel: () => void; onImport: () => void }) {
  return (
    <DialogFooter>
      <Button variant="ghost" onClick={props.onCancel}>
        {templatesCopy.cancel}
      </Button>
      <Button disabled={props.disabled} onClick={props.onImport}>
        {templatesCopy.importAction}
      </Button>
    </DialogFooter>
  );
}
