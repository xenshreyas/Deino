/**
 * The context rail's People: every speaker with a bar in their colour and
 * their share of the talk time, most first, computed from the transcript's
 * lines (`useTalkTime`). The bars are scaled to the top speaker, so the
 * differences read at a glance; nothing shows before anyone has spoken.
 */
import { SpeakerAvatar, speakerTone, talkTime } from '@/features/speakers';
import { type TalkShare, useTalkTime } from '@/features/transcript';
import { cn } from '@/ui/cn';
import { PanelSection } from '@/ui/panel-section';

import { meetingCopy } from '../copy';
import type { MeetingSlotProps } from '../types';

/** One speaker: avatar, name, bar and percent. */
function PersonRow({ share, top }: { share: TalkShare; top: number }) {
  const { speaker } = share;
  const width = top > 0 ? (share.ms / top) * 100 : 0;
  return (
    <li
      title={talkTime(share.ms)}
      className="grid grid-cols-[24px_minmax(0,1fr)_64px_32px] items-center gap-2"
    >
      <SpeakerAvatar name={speaker.name} color={speaker.color} unnamed={speaker.unnamed} />
      <span className="truncate text-[13px] text-fg">{speaker.name}</span>
      <span aria-hidden className="h-1 overflow-hidden rounded-full bg-bg-muted">
        <span
          className={cn('block h-full rounded-full', speakerTone(speaker.color).fill)}
          style={{ width: `${width.toFixed(1)}%` }}
        />
      </span>
      <span className="text-right text-xs text-fg-subtle tabular-nums">
        {meetingCopy.rail.percent(share.percent)}
      </span>
    </li>
  );
}

/** The section, or nothing before anyone has spoken. */
export function RailPeople({ meetingId }: MeetingSlotProps) {
  const shares = useTalkTime(meetingId);
  const top = shares[0]?.ms ?? 0;
  if (shares.length === 0) return null;
  return (
    <PanelSection
      title={meetingCopy.rail.people}
      action={<span className="text-xs text-fg-subtle">{meetingCopy.rail.talkTime}</span>}
    >
      <ul className="flex flex-col gap-2">
        {shares.map((share) => (
          <PersonRow key={share.speaker.key} share={share} top={top} />
        ))}
      </ul>
    </PanelSection>
  );
}
