/**
 * The meeting's clips, a section of its Details panel (TRN-04, UX-04): in
 * time order, each played through the meeting's player, renamed in place,
 * shown in Finder or deleted. Nothing shows until the meeting has a clip;
 * the transcript's selection menu makes one.
 */
import { PanelSection } from '@/ui/panel-section';

import { clipsCopy } from '../copy';
import { useMeetingClips } from '../queries';
import type { MeetingId } from '../types';
import { ClipRow } from './clip-row';

/** Props of {@link MeetingClips}. */
export interface MeetingClipsProps {
  meetingId: MeetingId;
}

/** The section, or nothing. */
export function MeetingClips({ meetingId }: MeetingClipsProps) {
  const clips = useMeetingClips(meetingId);
  const list = clips.data ?? [];
  if (list.length === 0) return null;
  return (
    <PanelSection title={clipsCopy.paletteHeading}>
      <ul aria-label={clipsCopy.paletteHeading} className="-mx-2 flex flex-col gap-0.5">
        {list.map((clip) => (
          <ClipRow key={clip.id} clip={clip} />
        ))}
      </ul>
    </PanelSection>
  );
}
