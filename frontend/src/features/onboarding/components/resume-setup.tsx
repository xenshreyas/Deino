import { Button } from '@/ui/button';
import { SkeletonText } from '@/ui/skeleton';

import { resumeCopy, stepNames } from '../copy';
import { unfinishedSteps } from '../machine';
import { useOnboardingDispatch, useOnboardingProgress } from '../queries';

/**
 * Settings → Setup: what first-run setup left undone, and the button that
 * reopens it at the first such step (the shell swaps the app for the flow).
 */
export function ResumeSetup() {
  const progress = useOnboardingProgress();
  const dispatch = useOnboardingDispatch();
  if (progress === undefined) return <SkeletonText lines={2} />;
  const left = unfinishedSteps(progress);
  return (
    <section className="flex flex-col items-start gap-4">
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
    </section>
  );
}
