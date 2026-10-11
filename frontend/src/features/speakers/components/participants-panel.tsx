/**
 * The meeting's Participants panel (CAP-04, ui.md §4.2 right panel): every
 * speaker with their colour, name, talk time and share, most speech first.
 * Naming, merging and answering "Is this …?" happen here; after a change
 * that reaches the notes, the panel offers to regenerate them.
 */
import { useState } from 'react';
import { toast } from 'sonner';

import { NOTES_STAGE, useRerunStage } from '@/features/pipeline';
import { Button } from '@/ui/button';
import { ICON_PROPS_LARGE, Users } from '@/ui/icons';
import { Skeleton } from '@/ui/skeleton';

import { announceRename } from '../announce';
import { speakersCopy } from '../copy';
import {
  useConfirmSuggestion,
  useDismissSuggestion,
  useMergeSpeakers,
  useParticipants,
} from '../queries';
import type { MeetingId, Participant } from '../types';
import { NotesStaleNotice } from './notes-stale-notice';
import { type ParticipantActions, ParticipantRow } from './participant-row';

/** Props of {@link ParticipantsPanel}. */
export interface ParticipantsPanelProps {
  meetingId: MeetingId;
}

function PanelSkeleton() {
  return (
    <div role="status" aria-label="Loading" className="flex flex-col gap-1 px-2 py-3">
      {Array.from({ length: 3 }, (_, index) => (
        <div key={index} className="flex items-start gap-3 px-2 py-2.5">
          <Skeleton className="size-6 rounded-full" />
          <div className="flex flex-1 flex-col gap-2.5 pt-1">
            <Skeleton className="h-2.5 w-1/2" />
            <Skeleton className="h-[3px] w-full" />
          </div>
        </div>
      ))}
    </div>
  );
}

function PanelEmpty() {
  return (
    <div className="flex flex-col items-start gap-2 px-4 py-6">
      <div className="flex size-9 items-center justify-center rounded-lg bg-bg-muted text-fg-muted">
        <Users {...ICON_PROPS_LARGE} size={18} />
      </div>
      <h3 className="pt-1 text-sm font-semibold text-fg">{speakersCopy.emptyTitle}</h3>
      <p className="text-sm text-pretty text-fg-muted">{speakersCopy.emptyBody}</p>
    </div>
  );
}

function PanelError({ onRetry }: { onRetry: () => void }) {
  return (
    <div className="flex flex-col items-start gap-2 px-4 py-6">
      <p className="text-sm text-danger">{speakersCopy.loadError}</p>
      <Button size="sm" variant="secondary" onClick={onRetry}>
        {speakersCopy.retry}
      </Button>
    </div>
  );
}

/** Merging and answering suggestions, with a flag for "the notes are stale". */
function usePanelActions(meetingId: MeetingId, markStale: () => void): ParticipantActions {
  const merge = useMergeSpeakers(meetingId);
  const confirm = useConfirmSuggestion(meetingId);
  const dismiss = useDismissSuggestion();
  const failed = (title: string) => (error: Error) =>
    toast.error(title, { description: error.message });
  return {
    merge: (from, into) => {
      merge.mutate(
        { from, into },
        { onSuccess: markStale, onError: failed(speakersCopy.mergeFailed) },
      );
    },
    confirm: (speakerId) => {
      confirm.mutate(speakerId, {
        onSuccess: (renamed) => {
          announceRename(renamed);
          markStale();
        },
        onError: failed(speakersCopy.answerFailed),
      });
    },
    dismiss: (speakerId) => {
      dismiss.mutate(speakerId, { onError: failed(speakersCopy.answerFailed) });
    },
    renamedAll: markStale,
    busy: merge.isPending || confirm.isPending || dismiss.isPending,
  };
}

/** Whether names changed since the notes were written, and re-running notes. */
function useStaleNotes(meetingId: MeetingId) {
  const [stale, setStale] = useState(false);
  const rerun = useRerunStage(meetingId);
  return {
    stale,
    regenerating: rerun.isPending,
    markStale: () => {
      setStale(true);
    },
    dismiss: () => {
      setStale(false);
    },
    regenerate: () => {
      rerun.mutate(NOTES_STAGE, {
        onSuccess: () => {
          toast.success(speakersCopy.regenerating);
          setStale(false);
        },
        onError: (error) =>
          toast.error(speakersCopy.regenerateFailed, { description: error.message }),
      });
    },
  };
}

/** The list, with the regenerate notice on top when names changed. */
function ParticipantList({
  meetingId,
  participants,
}: {
  meetingId: MeetingId;
  participants: readonly Participant[];
}) {
  const notes = useStaleNotes(meetingId);
  const actions = usePanelActions(meetingId, notes.markStale);
  return (
    <div className="flex flex-col py-3">
      {notes.stale ? (
        <NotesStaleNotice
          regenerating={notes.regenerating}
          onRegenerate={notes.regenerate}
          onDismiss={notes.dismiss}
        />
      ) : null}
      <ul aria-label={speakersCopy.panelLabel} className="flex flex-col px-2">
        {participants.map((participant) => (
          <ParticipantRow
            key={participant.speakerId}
            meetingId={meetingId}
            participant={participant}
            others={participants.filter((other) => other.speakerId !== participant.speakerId)}
            actions={actions}
          />
        ))}
      </ul>
    </div>
  );
}

/** The Participants panel of a meeting. */
export function ParticipantsPanel({ meetingId }: ParticipantsPanelProps) {
  const participants = useParticipants(meetingId);
  if (participants.isPending) return <PanelSkeleton />;
  if (participants.isError) {
    return (
      <PanelError
        onRetry={() => {
          void participants.refetch();
        }}
      />
    );
  }
  if (participants.data.length === 0) return <PanelEmpty />;
  return <ParticipantList meetingId={meetingId} participants={participants.data} />;
}
