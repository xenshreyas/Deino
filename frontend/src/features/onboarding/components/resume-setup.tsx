import { Button } from '@/ui/button';
import { SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { resumeCopy, stepNames } from '../copy';
import { unfinishedSteps } from '../machine';
import { useOnboardingDispatch, useOnboardingProgress } from '../queries';

/**
 * Settings → General → Setup: what first-run setup left undone, and the
 * button that reopens it at the first such step (the shell swaps the app
 * for the flow).
 */
export function ResumeSetup() {
  return (
    <SettingsSection title={resumeCopy.title} description={resumeCopy.description}>
      <ResumeBody />
    </SettingsSection>
  );
}

/** What is left, and the button. */
function ResumeBody() {
  const progress = useOnboardingProgress();
  const dispatch = useOnboardingDispatch();
  if (progress === undefined) return <SkeletonText lines={2} />;
  const left = unfinishedSteps(progress);
  return (
    <div className="flex flex-col items-start gap-3">
      <p className="text-sm text-fg-muted">
        {left.length === 0
          ? resumeCopy.allDone
          : resumeCopy.left(left.map((step) => stepNames[step]).join(', '))}
      </p>
      <Button
        variant="secondary"
        onClick={() => {
          dispatch({ type: 'resume' });
        }}
      >
        {left.length === 0 ? resumeCopy.restart : resumeCopy.resume}
      </Button>
    </div>
  );
}
