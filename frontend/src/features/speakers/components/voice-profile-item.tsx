/**
 * One person's voice profile in the list: name and sample count; opened, the
 * samples with where each came from and how clear it was, each deletable,
 * and "Forget …'s voice" for the whole profile.
 */
import { format } from 'date-fns';

import { AccordionContent, AccordionItem, AccordionTrigger } from '@/ui/accordion';
import { Button } from '@/ui/button';
import { IconButton } from '@/ui/icon-button';
import { Trash2 } from '@/ui/icons';

import { speakersCopy } from '../copy';
import type { VoiceprintId, VoiceprintSummary, VoiceProfile } from '../types';
import { SpeakerAvatar } from './speaker-avatar';

/** Props of {@link VoiceProfileItem}. */
export interface VoiceProfileItemProps {
  profile: VoiceProfile;
  onDeleteSample: (id: VoiceprintId) => void;
  onForget: (profile: VoiceProfile) => void;
}

/** `Oct 8`, or the raw text when it is not a date. */
function shortDate(iso: string): string {
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? iso : format(date, 'MMM d');
}

/** One sample: its meeting, date and clarity, and a delete button. */
function SampleRow({ sample, onDelete }: { sample: VoiceprintSummary; onDelete: () => void }) {
  const source =
    sample.sourceMeetingTitle === null
      ? speakersCopy.fromDeletedMeeting
      : speakersCopy.fromMeeting(sample.sourceMeetingTitle);
  return (
    <li className="flex h-10 items-center gap-3 pr-1 pl-9">
      <span className="min-w-0 flex-1 truncate text-sm text-fg">{source}</span>
      <span className="shrink-0 text-xs text-fg-muted tabular-nums">
        {shortDate(sample.createdAt)} · {speakersCopy.quality(Math.round(sample.quality * 100))}
      </span>
      <IconButton icon={Trash2} label={speakersCopy.deleteSample} onClick={onDelete} />
    </li>
  );
}

/** One person's collapsible profile. */
export function VoiceProfileItem({ profile, onDeleteSample, onForget }: VoiceProfileItemProps) {
  return (
    <AccordionItem value={profile.personId} className="px-4">
      <AccordionTrigger className="gap-3 py-3 font-normal">
        <span className="flex min-w-0 flex-1 items-center gap-3">
          <SpeakerAvatar name={profile.name} />
          <span className="truncate text-sm text-fg">{profile.name}</span>
          {profile.isMe ? <span className="text-xs text-fg-muted">{speakersCopy.you}</span> : null}
          <span className="ml-auto shrink-0 text-xs text-fg-muted tabular-nums">
            {speakersCopy.samples(profile.voiceprints.length)}
          </span>
        </span>
      </AccordionTrigger>
      <AccordionContent className="pb-3">
        <ul aria-label={profile.name}>
          {profile.voiceprints.map((sample) => (
            <SampleRow
              key={sample.id}
              sample={sample}
              onDelete={() => {
                onDeleteSample(sample.id);
              }}
            />
          ))}
        </ul>
        <Button
          size="sm"
          variant="danger"
          className="mt-1 ml-7"
          onClick={() => {
            onForget(profile);
          }}
        >
          {speakersCopy.forget(profile.name)}
        </Button>
      </AccordionContent>
    </AccordionItem>
  );
}
