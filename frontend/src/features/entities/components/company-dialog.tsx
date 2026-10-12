/**
 * Adds a company or edits one: its name and website. A pasted URL or an
 * address at the domain is read as the domain by Rust; a taken domain
 * shows under the form with the hint to merge instead.
 */
import { useNavigate } from '@tanstack/react-router';
import { type FormEvent, useState } from 'react';

import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/ui/dialog';

import { entitiesCopy } from '../copy';
import { useCreateCompany, useUpdateCompany } from '../mutations';
import type { Company, CompanyFields } from '../types';
import { FormError, FormFooter, TextField } from './form-parts';

/** What the dialog edits: a new company, or one; `null` while closed. */
export type CompanyDialogTarget = { kind: 'create' } | { kind: 'edit'; company: Company } | null;

type Target = Exclude<CompanyDialogTarget, null>;

/** Saves the form, then closes (and opens a new company's page). */
function useSaveCompany(target: Target, onClose: () => void) {
  const create = useCreateCompany();
  const update = useUpdateCompany();
  const navigate = useNavigate();
  const pending = target.kind === 'create' ? create : update;
  const save = (fields: CompanyFields) => {
    if (target.kind === 'edit') {
      update.mutate({ id: target.company.id, fields }, { onSuccess: onClose });
      return;
    }
    create.mutate(fields, {
      onSuccess: (company) => {
        onClose();
        void navigate({ to: '/companies/$companyId', params: { companyId: company.id } });
      },
    });
  };
  return { save, error: pending.error, isPending: pending.isPending };
}

/** The fields and actions; mounted per target so it starts from its values. */
function CompanyForm({ target, onClose }: { target: Target; onClose: () => void }) {
  const company = target.kind === 'edit' ? target.company : null;
  const [name, setName] = useState(company?.name ?? '');
  const [domain, setDomain] = useState(company?.domain ?? '');
  const { save, error, isPending } = useSaveCompany(target, onClose);
  const submit = (event: FormEvent) => {
    event.preventDefault();
    save({ name, domain });
  };
  return (
    <form onSubmit={submit} className="grid gap-4">
      <TextField
        id="company-name"
        label={entitiesCopy.nameLabel}
        value={name}
        placeholder={entitiesCopy.companyNamePlaceholder}
        autoFocus
        onChange={setName}
      />
      <TextField
        id="company-domain"
        label={entitiesCopy.domainLabel}
        value={domain}
        placeholder={entitiesCopy.domainPlaceholder}
        invalid={error !== null}
        onChange={setDomain}
      />
      <FormError error={error} />
      <FormFooter
        creating={target.kind === 'create'}
        disabled={name.trim() === '' || isPending}
        onClose={onClose}
      />
    </form>
  );
}

/** Adds or edits a company. */
export function CompanyDialog({
  target,
  onClose,
}: {
  target: CompanyDialogTarget;
  onClose: () => void;
}) {
  const title = target?.kind === 'edit' ? entitiesCopy.editCompanyTitle : entitiesCopy.companyTitle;
  return (
    <Dialog
      open={target !== null}
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <DialogContent aria-describedby={undefined}>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
        </DialogHeader>
        {target === null ? null : (
          <CompanyForm
            key={target.kind === 'edit' ? target.company.id : 'create'}
            target={target}
            onClose={onClose}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}
