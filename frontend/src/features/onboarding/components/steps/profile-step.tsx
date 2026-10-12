import { ProfilePage } from '@/features/profile';

import { profileCopy } from '../../copy';
import { StepFrame, type StepProps } from '../step-frame';

/** Step 5: who the user is, through Settings → Profile's form (NOT-05). */
export function ProfileStep({ dispatch }: StepProps) {
  return (
    <StepFrame
      step="profile"
      title={profileCopy.title}
      description={profileCopy.description}
      dispatch={dispatch}
    >
      <ProfilePage />
    </StepFrame>
  );
}
