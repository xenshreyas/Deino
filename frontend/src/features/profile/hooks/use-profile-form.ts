import { useState } from 'react';

import type { ProfileSettings } from '@/lib/ipc/generated/profile';

import { isDirty, type ProfileForm, toForm, toInput } from '../form';
import { useSaveProfile } from '../queries';

/** Binds one text field to the form: its value and change handler. */
export type Bind = (field: keyof ProfileForm) => {
  value: string;
  onChange: (event: { target: { value: string } }) => void;
};

/** The profile form's state, shared by Settings → Profile and onboarding. */
export interface ProfileFormState {
  /** Binds a field to the form. */
  bind: Bind;
  /** True when a field differs from the saved profile. */
  dirty: boolean;
  /** True while a save is in flight. */
  pending: boolean;
  /** The last save's error, in plain words. */
  error: string | null;
  /** Saves the form; `onSaved` runs once the profile is written. */
  save: (onSaved?: () => void) => void;
}

/**
 * Edits the profile as text, starting from `saved`, and saves it through the
 * `profile` plugin. A successful save resets the form to what Rust stored
 * (trimmed and de-duplicated).
 */
export function useProfileForm(saved: ProfileSettings): ProfileFormState {
  const [baseline, setBaseline] = useState(() => toForm(saved));
  const [form, setForm] = useState(baseline);
  const mutation = useSaveProfile();
  const bind: Bind = (field) => ({
    value: form[field],
    onChange: (event) => {
      setForm((current) => ({ ...current, [field]: event.target.value }));
    },
  });
  const save = (onSaved?: () => void) => {
    mutation.mutate(toInput(form), {
      onSuccess: (stored) => {
        const next = toForm(stored);
        setBaseline(next);
        setForm(next);
        onSaved?.();
      },
    });
  };
  return {
    bind,
    dirty: isDirty(form, baseline),
    pending: mutation.isPending,
    error: mutation.error?.message ?? null,
    save,
  };
}
