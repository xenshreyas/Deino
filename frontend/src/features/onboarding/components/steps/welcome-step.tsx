import { Check, ICON_PROPS } from '@/ui/icons';

import { flowCopy, welcomeCopy } from '../../copy';
import { StepFrame, type StepProps } from '../step-frame';

/** Step 1: what Deino is, and where its data goes. */
export function WelcomeStep({ dispatch }: StepProps) {
  return (
    <StepFrame
      step="welcome"
      title={welcomeCopy.title}
      description={welcomeCopy.description}
      dispatch={dispatch}
      mark={<AppIcon />}
      primary={{
        label: welcomeCopy.start,
        onClick: () => {
          dispatch({ type: 'continue' });
        },
      }}
      secondary={{
        label: flowCopy.skipSetup,
        onClick: () => {
          dispatch({ type: 'finish' });
        },
      }}
    >
      <ul className="flex flex-col gap-3">
        {welcomeCopy.points.map((point) => (
          <li key={point} className="flex items-start gap-3 text-sm text-fg">
            <Check {...ICON_PROPS} className="mt-0.5 shrink-0 text-accent" />
            {point}
          </li>
        ))}
      </ul>
    </StepFrame>
  );
}

/** The app icon, from `public/` (decorative: the title names the app). */
function AppIcon() {
  return <img src="/icon_128x128.png" alt="" width={56} height={56} className="-ml-1 size-14" />;
}
