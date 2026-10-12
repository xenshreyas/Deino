import { CalendarSettings } from '@/features/calendar';

import { calendarCopy } from '../../copy';
import { StepFrame, type StepProps } from '../step-frame';

/**
 * Step 4: macOS Calendar or a Google or Microsoft account, through the same
 * page Settings → Calendar shows (ORG-01), so there is one way to connect.
 */
export function CalendarStep({ dispatch }: StepProps) {
  return (
    <StepFrame
      step="calendar"
      title={calendarCopy.title}
      description={calendarCopy.description}
      dispatch={dispatch}
    >
      <CalendarSettings />
    </StepFrame>
  );
}
