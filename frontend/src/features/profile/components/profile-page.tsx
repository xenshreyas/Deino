import { type FormEvent, type ReactNode, useState } from 'react';
import { toast } from 'sonner';

import { useSettings } from '@/features/settings';
import { Button } from '@/ui/button';
import { Input, Textarea } from '@/ui/input';
import { SkeletonText } from '@/ui/skeleton';

import { profileCopy } from '../copy';
import { type ProfileForm, toForm, toInput } from '../form';
import { useProfileContext, useSaveProfile } from '../queries';

/**
 * Settings → Profile (NOT-05): who you are, for every AI prompt, and a
 * preview of exactly what the AI receives.
 */
export function ProfilePage() {
  const settings = useSettings();
  if (settings.data === undefined) return <SkeletonText lines={8} />;
  return (
    <div className="flex flex-col gap-8">
      <p className="text-sm text-fg-muted">{profileCopy.intro}</p>
      <ProfileFields initial={toForm(settings.data.profile)} />
      <ContextPreview />
    </div>
  );
}

/** The form, started from the saved profile. */
function ProfileFields({ initial }: { initial: ProfileForm }) {
  const [form, setForm] = useState(initial);
  const save = useSaveProfile();
  const bind: Bind = (field) => ({
    value: form[field],
    onChange: (event: { target: { value: string } }) => {
      setForm((current) => ({ ...current, [field]: event.target.value }));
    },
  });
  const submit = (event: FormEvent) => {
    event.preventDefault();
    save.mutate(toInput(form), {
      onSuccess: (saved) => {
        setForm(toForm(saved));
        toast.success(profileCopy.saved);
      },
    });
  };
  return (
    <form onSubmit={submit} className="flex flex-col gap-8">
      <YouFields bind={bind} />
      <WritingFields bind={bind} />
      <SaveRow pending={save.isPending} error={save.error?.message ?? null} />
    </form>
  );
}

/** Binds a text field to the form. */
type Bind = (field: keyof ProfileForm) => {
  value: string;
  onChange: (event: { target: { value: string } }) => void;
};

/** Name, addresses, role, company and focus areas. */
function YouFields({ bind }: { bind: Bind }) {
  return (
    <Group title={profileCopy.you}>
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
function WritingFields({ bind }: { bind: Bind }) {
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

function Group({ title, children }: { title: string; children: ReactNode }) {
  return (
    <fieldset className="flex flex-col gap-4">
      <legend className="pb-2 text-sm font-semibold text-fg">{title}</legend>
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

function SaveRow({ pending, error }: { pending: boolean; error: string | null }) {
  return (
    <div className="flex items-center justify-end gap-3">
      {error === null ? null : (
        <p role="alert" className="mr-auto text-sm text-danger">
          {error}
        </p>
      )}
      <Button type="submit" disabled={pending}>
        {pending ? profileCopy.saving : profileCopy.save}
      </Button>
    </div>
  );
}

/** The snippet every prompt starts with, exactly as sent. */
function ContextPreview() {
  const context = useProfileContext();
  return (
    <section aria-labelledby="profile-preview" className="flex flex-col gap-2">
      <h3 id="profile-preview" className="text-sm font-semibold text-fg">
        {profileCopy.preview}
      </h3>
      <p className="rounded-lg bg-bg-subtle px-4 py-3 text-sm leading-relaxed whitespace-pre-wrap text-fg">
        {context.data ?? profileCopy.previewEmpty}
      </p>
    </section>
  );
}
