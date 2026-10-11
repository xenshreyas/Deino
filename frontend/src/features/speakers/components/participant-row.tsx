/**
 * One speaker in the participants panel: avatar, name, talk time and share,
 * a talk-time bar in the speaker's colour, any "Is this …?" question, and
 * the hover menu. Clicking the name opens "Who is …?".
 */
import { useState } from 'react';

import { cn } from '@/ui/cn';
import { Popover, PopoverAnchor, PopoverContent, PopoverTrigger } from '@/ui/popover';

import { speakersCopy } from '../copy';
import { displayName, speakerTone, talkTime } from '../format';
import type { MeetingId, Participant, SpeakerId } from '../types';
import { AssignSpeaker } from './assign-speaker';
import { ParticipantMenu } from './participant-menu';
import { SpeakerAvatar } from './speaker-avatar';
import { SuggestionPrompt } from './suggestion-prompt';

/** What a row can ask the panel to do. */
export interface ParticipantActions {
  merge: (from: SpeakerId, into: SpeakerId) => void;
  confirm: (speakerId: SpeakerId) => void;
  dismiss: (speakerId: SpeakerId) => void;
  /** After any rename that covered all segments. */
  renamedAll: () => void;
  /** True while an answer or a merge is being saved. */
  busy: boolean;
}

/** Props of {@link ParticipantRow}. */
export interface ParticipantRowProps {
  meetingId: MeetingId;
  participant: Participant;
  others: readonly Participant[];
  actions: ParticipantActions;
}

/** True while nobody is assigned to the voice. */
function isUnnamed(participant: Participant): boolean {
  return participant.person === null && !participant.isMe;
}

/** The share of the meeting as a thin bar in the speaker's colour. */
function TalkBar({ percent, color }: { percent: number; color: number }) {
  return (
    <div aria-hidden className="mt-2 h-[3px] overflow-hidden rounded-full bg-bg-muted">
      <div
        className={cn(
          'h-full rounded-full transition-[width] duration-(--dur-slow) ease-out',
          speakerTone(color).fill,
        )}
        style={{ width: `${String(Math.max(percent, 1))}%` }}
      />
    </div>
  );
}

/** Name (opening the rename popover) and the talk time. */
function NameLine({ participant }: { participant: Participant }) {
  const name = displayName(participant);
  const time = talkTime(participant.speech);
  return (
    <div className="flex items-baseline gap-2">
      <PopoverTrigger
        className={cn(
          'min-w-0 truncate rounded-xs text-left text-sm outline-offset-2 hover:underline hover:underline-offset-4',
          isUnnamed(participant) ? 'text-fg-muted' : 'text-fg',
        )}
      >
        {name}
      </PopoverTrigger>
      <span
        className="ml-auto shrink-0 text-xs text-fg-muted tabular-nums"
        aria-label={speakersCopy.talkTime(name, time, participant.sharePercent)}
      >
        {time} · {speakersCopy.share(participant.sharePercent)}
      </span>
    </div>
  );
}

/** Name, bar and the "Is this …?" question. */
function RowDetails({
  participant,
  actions,
}: {
  participant: Participant;
  actions: ParticipantActions;
}) {
  const { speakerId, suggestion } = participant;
  return (
    <div className="min-w-0 flex-1">
      <NameLine participant={participant} />
      <TalkBar percent={participant.sharePercent} color={participant.color} />
      {suggestion === null ? null : (
        <SuggestionPrompt
          suggestion={suggestion}
          busy={actions.busy}
          onConfirm={() => {
            actions.confirm(speakerId);
          }}
          onDismiss={() => {
            actions.dismiss(speakerId);
          }}
        />
      )}
    </div>
  );
}

/** The "Who is …?" popover of one participant. */
function RenamePopover({
  meetingId,
  participant,
  onRenamedAll,
  onClose,
}: {
  meetingId: MeetingId;
  participant: Participant;
  onRenamedAll: () => void;
  onClose: () => void;
}) {
  return (
    <PopoverContent align="start" className="w-72 p-0">
      <AssignSpeaker
        meetingId={meetingId}
        speakerId={participant.speakerId}
        name={displayName(participant)}
        onRenamed={(_, scope) => {
          if (scope === 'all') onRenamedAll();
        }}
        onDone={onClose}
      />
    </PopoverContent>
  );
}

/** One participant, anchoring its "Who is …?" popover. */
export function ParticipantRow({ meetingId, participant, others, actions }: ParticipantRowProps) {
  const [open, setOpen] = useState(false);
  const name = displayName(participant);
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverAnchor asChild>
        <li className="group/row flex items-start gap-3 rounded-md px-2 py-2.5 transition-colors duration-(--dur-fast) hover:bg-bg-muted/60">
          <SpeakerAvatar
            name={name}
            color={participant.color}
            unnamed={isUnnamed(participant)}
            className="mt-px"
          />
          <RowDetails participant={participant} actions={actions} />
          <ParticipantMenu
            participant={participant}
            others={others}
            onRename={() => {
              setOpen(true);
            }}
            onMerge={(into) => {
              actions.merge(participant.speakerId, into);
            }}
          />
        </li>
      </PopoverAnchor>
      <RenamePopover
        meetingId={meetingId}
        participant={participant}
        onRenamedAll={actions.renamedAll}
        onClose={() => {
          setOpen(false);
        }}
      />
    </Popover>
  );
}
