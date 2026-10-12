/**
 * The profile form's values as typed, and their conversions. Lists are
 * edited as text (emails one per line, focus areas comma-separated); Rust
 * trims, lower-cases and de-duplicates on save.
 */
import type { ProfileInput, ProfileSettings } from '@/lib/ipc/generated/profile';

/** The form's fields, as text. */
export interface ProfileForm {
  name: string;
  emails: string;
  role: string;
  company: string;
  focusAreas: string;
  writingStyle: string;
  context: string;
}

/** Every field of the form, for comparing two forms. */
const FIELDS = [
  'name',
  'emails',
  'role',
  'company',
  'focusAreas',
  'writingStyle',
  'context',
] as const satisfies readonly (keyof ProfileForm)[];

/** True when any field of `form` differs from `saved`. */
export function isDirty(form: ProfileForm, saved: ProfileForm): boolean {
  return FIELDS.some((field) => form[field] !== saved[field]);
}

/** The form for a saved profile. */
export function toForm(profile: ProfileSettings): ProfileForm {
  return {
    name: profile.name,
    emails: profile.emails.join('\n'),
    role: profile.role,
    company: profile.company,
    focusAreas: profile.focusAreas.join(', '),
    writingStyle: profile.writingStyle,
    context: profile.context,
  };
}

/** What the `profile.update` command takes. */
export function toInput(form: ProfileForm): ProfileInput {
  return {
    name: form.name,
    emails: form.emails.split(/[\n,;]+/).map((email) => email.trim()),
    role: form.role,
    company: form.company,
    focusAreas: form.focusAreas.split(',').map((area) => area.trim()),
    writingStyle: form.writingStyle,
    context: form.context,
  };
}
