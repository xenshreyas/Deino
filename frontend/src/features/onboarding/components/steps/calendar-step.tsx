import { CalendarSettings } from '@/features/calendar';

import { calendarCopy } from '../../copy';
import { StepFrame, type StepProps } from '../step-frame';

/**
 * Step 4: macOS Calendar, through the same page Settings → Calendar shows
 * (ORG-01), so there is one way to connect. Direct Google and Microsoft
 * connections need an app registration, so they wait for Settings.
 */
export function CalendarStep({ dispatch }: StepProps) {
  return (
    <StepFrame
      step="calendar"
      title={calendarCopy.title}
      description={calendarCopy.description}
      dispatch={dispatch}
    >
      <CalendarSettings connectors={false} />
      <p className="text-xs text-fg-muted">{calendarCopy.more}</p>
    </StepFrame>
  );
}
