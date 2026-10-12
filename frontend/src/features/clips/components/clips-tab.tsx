/**
 * The meeting's Clips tab (TRN-04): its clips in time order, each played
 * through the meeting's player, renamed in place, shown in Finder or
 * deleted. Empty, it says how to make one.
 */
import { EmptyState } from '@/ui/empty-state';
import { Scissors } from '@/ui/icons';
import { Skeleton } from '@/ui/skeleton';

import { clipsCopy } from '../copy';
import { useMeetingClips } from '../queries';
import type { MeetingId } from '../types';
import { ClipRow } from './clip-row';

/** Props of {@link ClipsTab}. */
export interface ClipsTabProps {
  meetingId: MeetingId;
}

/** The tab. */
export function ClipsTab({ meetingId }: ClipsTabProps) {
  const clips = useMeetingClips(meetingId);
  if (clips.isPending) {
    return (
      <div className="mx-auto flex w-full max-w-[720px] flex-col gap-2 px-6 pt-6">
        <Skeleton className="h-11" />
        <Skeleton className="h-11" />
      </div>
    );
  }
  const list = clips.data ?? [];
  if (list.length === 0) {
    return (
      <EmptyState
        className="pt-[12vh]"
        icon={Scissors}
        title={clipsCopy.tabEmptyTitle}
        description={clipsCopy.tabEmptyBody}
      />
    );
  }
  return (
    <ul
      aria-label={clipsCopy.paletteHeading}
      className="mx-auto flex w-full max-w-[720px] flex-col gap-0.5 px-4 pt-4"
    >
      {list.map((clip) => (
        <ClipRow key={clip.id} clip={clip} />
      ))}
    </ul>
  );
}
