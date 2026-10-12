import type { FormEvent } from 'react';

import { useSettings } from '@/features/settings';
import type { ProfileSettings } from '@/lib/ipc/generated/profile';
import { Button } from '@/ui/button';
import { SkeletonText } from '@/ui/skeleton';

import { profileCopy } from '../copy';
import { useProfileForm } from '../hooks/use-profile-form';
import { useProfileContext } from '../queries';
import { AboutYouFields, WritingFields } from './profile-fields';

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
      <ProfileFields saved={settings.data.profile} />
      <ContextPreview />
    </div>
  );
}

/** The form, started from the saved profile. */
function ProfileFields({ saved }: { saved: ProfileSettings }) {
  const profile = useProfileForm(saved);
  const submit = (event: FormEvent) => {
    event.preventDefault();
    // Saved silently (UX-31); a failure shows under the form.
    profile.save(() => undefined);
  };
  return (
    <form onSubmit={submit} className="flex flex-col gap-8">
      <AboutYouFields bind={profile.bind} />
      <WritingFields bind={profile.bind} />
      <SaveRow pending={profile.pending} error={profile.error} />
    </form>
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
