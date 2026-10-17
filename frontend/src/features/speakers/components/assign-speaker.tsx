/**
 * "Who is Speaker 2?" (ui.md §3 Speaker chip → "Assign speaker"): pick an
 * attendee, anyone in People, or add someone new. Opened from the
 * participants panel it renames every segment of the speaker (which also
 * saves their voice); opened from a transcript segment (TRN-01) it offers
 * "This segment" or "All segments".
 */
import { useState } from 'react';
import { toast } from 'sonner';

import { SegmentedControl } from '@/ui/segmented-control';

import { announceRename } from '../announce';
import { speakersCopy } from '../copy';
import { useNameCandidates, useRenameSpeaker } from '../queries';
import type { MeetingId, Renamed, RenameScope, SegmentId, SpeakerId, SpeakerName } from '../types';
import { NamePicker } from './name-picker';

/** The scope choice, as segment values. */
type ScopeChoice = 'segment' | 'all';

/** Props of {@link AssignSpeaker}. */
export interface AssignSpeakerProps {
  meetingId: MeetingId;
  speakerId: SpeakerId;
  /** How the speaker reads now ("Speaker 2"). */
  name: string;
  /** The segment it was opened from; offers "This segment". */
  segmentId?: SegmentId | undefined;
  /** After a successful rename, with the scope it covered. */
  onRenamed?: ((renamed: Renamed, scope: RenameScope['kind']) => void) | undefined;
  /** Closes the popover. */
  onDone: () => void;
}

/** The scope a choice stands for. */
function scopeOf(choice: ScopeChoice, segmentId: SegmentId | undefined): RenameScope {
  return choice === 'segment' && segmentId !== undefined
    ? { kind: 'segment', segmentId }
    : { kind: 'all' };
}

/**
 * Renames the speaker as the person picked, in the chosen scope, and closes.
 * The outcome is handled on the promise, not `mutate`'s callbacks: closing
 * unmounts this popover, and TanStack Query drops a gone observer's callbacks.
 */
function usePick(props: AssignSpeakerProps, choice: ScopeChoice) {
  const { meetingId, speakerId, segmentId, onRenamed, onDone } = props;
  const rename = useRenameSpeaker();
  return (person: SpeakerName) => {
    const scope = scopeOf(choice, segmentId);
    rename.mutateAsync({ meetingId, speakerId, name: person, scope }).then(
      (renamed) => {
        announceRename(renamed);
        onRenamed?.(renamed, scope.kind);
      },
      (error: unknown) => {
        const description = error instanceof Error ? error.message : undefined;
        toast.error(speakersCopy.renameFailed, { description });
      },
    );
    onDone();
  };
}

/** The popover body: title, optional scope, the name list. */
export function AssignSpeaker(props: AssignSpeakerProps) {
  const { meetingId, name, segmentId } = props;
  const [search, setSearch] = useState('');
  // Naming a speaker names all their lines unless the user narrows it.
  const [choice, setChoice] = useState<ScopeChoice>('all');
  const candidates = useNameCandidates(meetingId, search);
  const pick = usePick(props, choice);
  return (
    <div className="flex flex-col">
      <div className="flex items-center justify-between gap-3 px-3 pt-3 pb-2">
        <h3 className="truncate text-sm font-semibold text-fg">{speakersCopy.assignTitle(name)}</h3>
        {segmentId === undefined ? null : (
          <SegmentedControl<ScopeChoice>
            label={speakersCopy.scopeLabel}
            value={choice}
            onValueChange={setChoice}
            segments={[
              { value: 'all', label: speakersCopy.scopeAll },
              { value: 'segment', label: speakersCopy.scopeSegment },
            ]}
          />
        )}
      </div>
      <NamePicker
        candidates={candidates.data ?? []}
        search={search}
        onSearch={setSearch}
        onPick={pick}
      />
    </div>
  );
}
