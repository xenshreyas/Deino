/**
 * Adds a person or edits one: name, email, title and company. Errors from
 * Rust (a blank name, a bad or taken address) show under the form.
 */
import { useNavigate } from '@tanstack/react-router';
import { type FormEvent, useState } from 'react';

import { Label } from '@/ui/controls';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/ui/dialog';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/ui/select';

import { entitiesCopy } from '../copy';
import { useCreatePerson, useUpdatePerson } from '../mutations';
import { useCompanies } from '../queries';
import type { CompanyId, Person, PersonFields } from '../types';
import { FormError, FormFooter, TextField } from './form-parts';

/** Who the dialog edits: nobody yet (add), or a person; `null` while closed. */
export type PersonDialogTarget = { kind: 'create' } | { kind: 'edit'; person: Person } | null;

type Target = Exclude<PersonDialogTarget, null>;

/** The select's value for "no company". */
const NO_COMPANY = 'none';

function initialFields(target: Target): PersonFields {
  if (target.kind === 'create') return { name: '', email: '', title: '', companyId: null };
  const { name, email, title, companyId } = target.person;
  return { name, email: email ?? '', title: title ?? '', companyId };
}

/** Saves the form, then closes (and opens a new person's page). */
function useSavePerson(target: Target, onClose: () => void) {
  const create = useCreatePerson();
  const update = useUpdatePerson();
  const navigate = useNavigate();
  const pending = target.kind === 'create' ? create : update;
  const save = (fields: PersonFields) => {
    if (target.kind === 'edit') {
      update.mutate({ id: target.person.id, fields }, { onSuccess: onClose });
      return;
    }
    create.mutate(fields, {
      onSuccess: (person) => {
        onClose();
        void navigate({ to: '/people/$personId', params: { personId: person.id } });
      },
    });
  };
  return { save, error: pending.error, isPending: pending.isPending };
}

/** The company picker: every company, or none. */
function CompanyField({
  value,
  onChange,
}: {
  value: CompanyId | null;
  onChange: (id: CompanyId | null) => void;
}) {
  const companies = useCompanies().data ?? [];
  return (
    <div className="grid gap-1.5">
      <Label htmlFor="person-company">{entitiesCopy.companyLabel}</Label>
      <Select
        value={value ?? NO_COMPANY}
        onValueChange={(next) => {
          onChange(companies.find((company) => company.id === next)?.id ?? null);
        }}
      >
        <SelectTrigger id="person-company">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value={NO_COMPANY}>{entitiesCopy.noCompany}</SelectItem>
          {companies.map((company) => (
            <SelectItem key={company.id} value={company.id}>
              {company.name}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}

/** Name, email and title. */
function TextFields({
  fields,
  set,
}: {
  fields: PersonFields;
  set: (patch: Partial<PersonFields>) => void;
}) {
  const bind = (key: 'name' | 'email' | 'title') => (value: string) => {
    set({ [key]: value });
  };
  return (
    <>
      <TextField
        id="person-name"
        label={entitiesCopy.nameLabel}
        value={fields.name}
        placeholder={entitiesCopy.personNamePlaceholder}
        autoFocus
        onChange={bind('name')}
      />
      <TextField
        id="person-email"
        type="email"
        label={entitiesCopy.emailLabel}
        value={fields.email ?? ''}
        placeholder={entitiesCopy.emailPlaceholder}
        onChange={bind('email')}
      />
      <TextField
        id="person-title"
        label={entitiesCopy.jobTitleLabel}
        value={fields.title ?? ''}
        placeholder={entitiesCopy.jobTitlePlaceholder}
        onChange={bind('title')}
      />
    </>
  );
}

/** The fields and actions; mounted per target so it starts from its values. */
function PersonForm({ target, onClose }: { target: Target; onClose: () => void }) {
  const [fields, setFields] = useState(() => initialFields(target));
  const { save, error, isPending } = useSavePerson(target, onClose);
  const set = (patch: Partial<PersonFields>) => {
    setFields((current) => ({ ...current, ...patch }));
  };
  const submit = (event: FormEvent) => {
    event.preventDefault();
    save(fields);
  };
  return (
    <form onSubmit={submit} className="grid gap-4">
      <TextFields fields={fields} set={set} />
      <CompanyField
        value={fields.companyId}
        onChange={(companyId) => {
          set({ companyId });
        }}
      />
      <FormError error={error} />
      <FormFooter
        creating={target.kind === 'create'}
        disabled={fields.name.trim() === '' || isPending}
        onClose={onClose}
      />
    </form>
  );
}

/** Adds or edits a person. */
export function PersonDialog({
  target,
  onClose,
}: {
  target: PersonDialogTarget;
  onClose: () => void;
}) {
  const title = target?.kind === 'edit' ? entitiesCopy.editPersonTitle : entitiesCopy.personTitle;
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
          <PersonForm
            key={target.kind === 'edit' ? target.person.id : 'create'}
            target={target}
            onClose={onClose}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}
