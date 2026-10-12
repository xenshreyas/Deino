import type { FormEvent } from 'react';

import { AboutYouFields, useProfileForm } from '@/features/profile';
import { useSettings } from '@/features/settings';
import type { ProfileSettings } from '@/lib/ipc/generated/profile';
import { SkeletonText } from '@/ui/skeleton';

import { flowCopy, profileCopy } from '../../copy';
import { StepFrame, type StepProps } from '../step-frame';

/** Step 5: who the user is (NOT-05), once the saved profile has loaded. */
export function ProfileStep({ dispatch }: StepProps) {
  const saved = useSettings().data?.profile;
  if (saved === undefined)
    return (
      <StepFrame
        step="profile"
        title={profileCopy.title}
        description={profileCopy.description}
        dispatch={dispatch}
      >
        <SkeletonText lines={6} />
      </StepFrame>
    );
  return <ProfileForm saved={saved} dispatch={dispatch} />;
}

/**
 * The "About you" half of Settings → Profile. Continue (or Return in a
 * field) saves any edits first, so nothing typed here is lost; with no
 * edits it just moves on. Writing style and the glossary stay in Settings.
 */
function ProfileForm({ saved, dispatch }: StepProps & { saved: ProfileSettings }) {
  const profile = useProfileForm(saved);
  const next = () => {
    dispatch({ type: 'continue' });
  };
  const saveAndContinue = () => {
    if (profile.dirty) profile.save(next);
    else next();
  };
  const submit = (event: FormEvent) => {
    event.preventDefault();
    saveAndContinue();
  };
  return (
    <StepFrame
      step="profile"
      title={profileCopy.title}
      description={profileCopy.description}
      dispatch={dispatch}
      primary={{
        label: profile.pending ? profileCopy.saving : flowCopy.continue,
        onClick: saveAndContinue,
        disabled: profile.pending,
      }}
    >
      <form onSubmit={submit} className="flex flex-col gap-6" aria-label={profileCopy.title}>
        <AboutYouFields bind={profile.bind} legend={false} />
        {/* Return in a single-line field submits; the button stays hidden. */}
        <button type="submit" hidden />
      </form>
      {profile.error === null ? null : (
        <p role="alert" className="text-sm text-danger">
          {profileCopy.saveFailed} {profile.error}
        </p>
      )}
      <p className="text-xs text-fg-muted">{profileCopy.more}</p>
    </StepFrame>
  );
}
