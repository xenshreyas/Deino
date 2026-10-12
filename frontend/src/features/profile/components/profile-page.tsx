import type { FormEvent } from 'react';
import { toast } from 'sonner';

import { useSettings } from '@/features/settings';
import type { ProfileSettings } from '@/lib/ipc/generated/profile';
import { Button } from '@/ui/button';
import { SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { profileCopy } from '../copy';
import { useProfileForm } from '../hooks/use-profile-form';
import { useProfileContext } from '../queries';
import { AboutYouFields, WritingFields } from './profile-fields';

/**
 * Settings → Notes → About you (NOT-05): who you are, for every AI prompt,
 * and a preview of exactly what the AI receives.
 */
export function ProfilePage() {
  const settings = useSettings();
  return (
    <SettingsSection title={profileCopy.title} description={profileCopy.intro}>
      {settings.data === undefined ? (
        <SkeletonText lines={8} />
      ) : (
        <div className="flex flex-col gap-8 pt-2">
          <ProfileFields saved={settings.data.profile} />
          <ContextPreview />
        </div>
      )}
    </SettingsSection>
  );
}

/** The form, started from the saved profile. */
function ProfileFields({ saved }: { saved: ProfileSettings }) {
  const profile = useProfileForm(saved);
  const submit = (event: FormEvent) => {
    event.preventDefault();
    profile.save(() => toast.success(profileCopy.saved));
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
      <h4 id="profile-preview" className="text-sm font-semibold text-fg">
        {profileCopy.preview}
      </h4>
      <p className="rounded-lg bg-bg-subtle px-4 py-3 text-sm leading-relaxed whitespace-pre-wrap text-fg">
        {context.data ?? profileCopy.previewEmpty}
      </p>
    </section>
  );
}
