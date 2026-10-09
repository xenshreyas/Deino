/**
 * The attendee-domain rule form: "Add attendees from a domain". Rust
 * normalises what is typed (`@Acme.com` becomes `acme.com`) and refuses
 * anything that is not a domain.
 */
import { type FormEvent, useState } from 'react';

import { Button } from '@/ui/button';
import { Label } from '@/ui/controls';
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/ui/dialog';
import { Input } from '@/ui/input';

import { foldersCopy } from '../copy';

/** Props of {@link DomainDialog}. */
export interface DomainDialogProps {
  open: boolean;
  onClose: () => void;
  onAdd: (domain: string) => void;
}

/** The field and actions; mounted while open so it starts empty. */
function DomainForm({ onClose, onAdd }: Omit<DomainDialogProps, 'open'>) {
  const [domain, setDomain] = useState('');
  const submit = (event: FormEvent) => {
    event.preventDefault();
    onAdd(domain);
  };
  return (
    <form onSubmit={submit} className="grid gap-4">
      <div className="grid gap-1.5">
        <Label htmlFor="rule-domain">{foldersCopy.domainLabel}</Label>
        <Input
          id="rule-domain"
          value={domain}
          placeholder={foldersCopy.domainPlaceholder}
          autoFocus
          onChange={(event) => {
            setDomain(event.target.value);
          }}
        />
      </div>
      <DialogFooter>
        <Button variant="ghost" type="button" onClick={onClose}>
          {foldersCopy.cancel}
        </Button>
        <Button type="submit" disabled={domain.trim() === ''}>
          {foldersCopy.add}
        </Button>
      </DialogFooter>
    </form>
  );
}

/** Asks for an email domain and adds it as a rule. */
export function DomainDialog({ open, onClose, onAdd }: DomainDialogProps) {
  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) onClose();
      }}
    >
      <DialogContent aria-describedby={undefined}>
        <DialogHeader>
          <DialogTitle>{foldersCopy.domainTitle}</DialogTitle>
        </DialogHeader>
        {open ? <DomainForm onClose={onClose} onAdd={onAdd} /> : null}
      </DialogContent>
    </Dialog>
  );
}
