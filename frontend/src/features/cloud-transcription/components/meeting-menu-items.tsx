/**
 * A meeting's ⋯ menu entries for ElevenLabs (CAP-14): "Re-transcribe with
 * ElevenLabs" once a key is saved, and "Restore local transcript" when the
 * transcript made on this Mac was kept. Nothing shows otherwise.
 *
 * Self-contained (it takes only the meeting ID), so the ⋯ menu's
 * re-transcribe actions (`features/meeting/components/recording-menu-items.tsx`,
 * UX-05) add it with one line next to "Re-transcribe…":
 * `<CloudTranscriptMenuItems meetingId={meetingId} />`.
 */
import { toast } from 'sonner';

import { AudioLines, ICON_PROPS, Undo2 } from '@/ui/icons';
import { DropdownMenuItem } from '@/ui/menu';

import { cloudCopy } from '../copy';
import {
  useElevenLabsKey,
  useHasOriginalTranscript,
  useRestoreOriginal,
  useRetranscribe,
} from '../queries';
import type { MeetingId } from '../types';

/** Props of the menu entries. */
interface MenuItemsProps {
  meetingId: MeetingId;
}

/** Sends the meeting to ElevenLabs again; its steps show the progress. */
function RetranscribeItem({ meetingId }: MenuItemsProps) {
  const retranscribe = useRetranscribe(meetingId);
  return (
    <DropdownMenuItem
      disabled={retranscribe.isPending}
      onSelect={() => {
        retranscribe.mutate(undefined, {
          onError: (error) =>
            toast.error(cloudCopy.retranscribeFailed, { description: error.message }),
        });
      }}
    >
      <AudioLines {...ICON_PROPS} />
      {cloudCopy.retranscribe}
    </DropdownMenuItem>
  );
}

/** Puts the transcript made on this Mac back. */
function RestoreItem({ meetingId }: MenuItemsProps) {
  const restore = useRestoreOriginal(meetingId);
  return (
    <DropdownMenuItem
      disabled={restore.isPending}
      onSelect={() => {
        restore.mutate(undefined, {
          onError: (error) => toast.error(cloudCopy.restoreFailed, { description: error.message }),
        });
      }}
    >
      <Undo2 {...ICON_PROPS} />
      {cloudCopy.restore}
    </DropdownMenuItem>
  );
}

/** The entries that apply to this meeting; nothing when none does. */
export function CloudTranscriptMenuItems({ meetingId }: MenuItemsProps) {
  const key = useElevenLabsKey();
  const original = useHasOriginalTranscript(meetingId);
  const canSend = key.data === 'stored';
  const canRestore = original.data === true;
  if (!canSend && !canRestore) return null;
  return (
    <>
      {canSend ? <RetranscribeItem meetingId={meetingId} /> : null}
      {canRestore ? <RestoreItem meetingId={meetingId} /> : null}
    </>
  );
}
