/**
 * The context rail: a quiet 300px column beside the meeting's 720px column,
 * in the right gutter a wide window leaves empty. Its sections are the
 * `MEETING_RAIL` slots (people with talk time, action items, the follow-up
 * draft). It shows only while the meeting area is at least 1100px wide (a
 * container query on `@container/meeting`) and the right panel is closed,
 * since Details shows the same people and the follow-up; narrower, the page
 * is exactly the column.
 */
import { useShellLayout } from '@/features/shell';
import type { MeetingId } from '@/features/meetings';
import { cn } from '@/ui/cn';

import { meetingCopy } from '../copy';
import { MEETING_RAIL } from '../slots';

/** The width of the meeting area from which the rail shows. */
export const RAIL_MIN_AREA = 1100;

/** Hidden until the meeting area is {@link RAIL_MIN_AREA} wide. */
const SHOW_WHEN_WIDE = '@min-[1100px]/meeting:flex';

/** The rail, or nothing while the right panel is open. */
export function ContextRail({ meetingId }: { meetingId: MeetingId }) {
  const rightPanelOpen = useShellLayout((state) => state.rightPanelOpen);
  if (rightPanelOpen) return null;
  return (
    <aside
      aria-label={meetingCopy.rail.label}
      data-chrome
      className={cn(
        'hidden w-[300px] shrink-0 flex-col gap-[22px] overflow-y-auto pt-8 pb-10',
        SHOW_WHEN_WIDE,
      )}
    >
      {MEETING_RAIL.map(({ id, Component }) => (
        <Component key={id} meetingId={meetingId} />
      ))}
    </aside>
  );
}
