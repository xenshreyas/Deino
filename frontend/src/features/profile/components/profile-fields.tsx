import type { ReactNode } from 'react';

import { Input, Textarea } from '@/ui/input';

import { profileCopy } from '../copy';
import type { Bind } from '../hooks/use-profile-form';

/** Name, addresses, role, company and focus areas. */
export function AboutYouFields({ bind, legend = true }: { bind: Bind; legend?: boolean }) {
  return (
    <Group title={legend ? profileCopy.you : null}>
      <Field label={profileCopy.name}>
        <Input placeholder={profileCopy.namePlaceholder} autoComplete="name" {...bind('name')} />
      </Field>
      <Field label={profileCopy.emails} hint={profileCopy.emailsHint}>
        <Textarea rows={2} autoComplete="email" {...bind('emails')} />
      </Field>
      <div className="grid grid-cols-2 gap-4">
        <Field label={profileCopy.role}>
          <Input placeholder={profileCopy.rolePlaceholder} {...bind('role')} />
        </Field>
        <Field label={profileCopy.company}>
          <Input autoComplete="organization" {...bind('company')} />
        </Field>
      </div>
      <Field label={profileCopy.focus} hint={profileCopy.focusHint}>
        <Input placeholder={profileCopy.focusPlaceholder} {...bind('focusAreas')} />
      </Field>
    </Group>
  );
}

/** Writing style and the glossary. */
export function WritingFields({ bind }: { bind: Bind }) {
  return (
    <Group title={profileCopy.writing}>
      <Field label={profileCopy.writingStyle}>
        <Textarea
          rows={2}
          placeholder={profileCopy.writingStylePlaceholder}
          {...bind('writingStyle')}
        />
      </Field>
      <Field label={profileCopy.context} hint={profileCopy.contextHint}>
        <Textarea rows={5} placeholder={profileCopy.contextPlaceholder} {...bind('context')} />
      </Field>
    </Group>
  );
}

function Group({ title, children }: { title: string | null; children: ReactNode }) {
  return (
    <fieldset className="flex flex-col gap-4">
      {title === null ? null : (
        <legend className="pb-2 text-sm font-semibold text-fg">{title}</legend>
      )}
      {children}
    </fieldset>
  );
}

function Field({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <label className="flex flex-col gap-1.5">
      <span className="text-sm text-fg">{label}</span>
      {children}
      {hint === undefined ? null : <span className="text-xs text-fg-muted">{hint}</span>}
    </label>
  );
}
