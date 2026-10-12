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
