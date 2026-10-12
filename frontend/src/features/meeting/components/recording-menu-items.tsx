/**
 * The ⋯ menu's recording actions (UX-05): re-transcribe the recording and
 * show it in Finder. Both need the recording folder, which the inherited
 * session publishes (`legacy/panels-context.tsx`); they stay disabled until
 * it has loaded, and for a meeting with no recording on this Mac.
 *
 * Every re-transcribe action lives here, one item each, so another engine
 * (for example "Re-transcribe with ElevenLabs") slots in next to
 * "Re-transcribe…".
 */
import { RetranscribeDialog } from '@/components/MeetingDetails/RetranscribeDialog';
import { FolderOpen, ICON_PROPS, RotateCw } from '@/ui/icons';
import { DropdownMenuItem } from '@/ui/menu';

import { meetingCopy } from '../copy';
import { type LegacyRecording, useLegacyMeetingPanels } from '../legacy/panels-context';

/** The recording, once the inherited session knows it has one on this Mac. */
export function useRecording(): LegacyRecording | null {
  const legacy = useLegacyMeetingPanels();
  if (legacy.status !== 'ready' || legacy.recording.folderPath === null) return null;
  return legacy.recording;
}

/** Props of {@link RecordingMenuItems}. */
export interface RecordingMenuItemsProps {
  recording: LegacyRecording | null;
  onRetranscribe: () => void;
}

/** "Re-transcribe…" and "Show recording in Finder". */
export function RecordingMenuItems({ recording, onRetranscribe }: RecordingMenuItemsProps) {
  const missing = recording === null;
  return (
    <>
      <DropdownMenuItem disabled={missing} onSelect={onRetranscribe}>
        <RotateCw {...ICON_PROPS} />
        {meetingCopy.retranscribe}
      </DropdownMenuItem>
      <DropdownMenuItem
        disabled={missing}
        onSelect={() => {
          void recording?.openFolder();
        }}
      >
        <FolderOpen {...ICON_PROPS} />
        {meetingCopy.showRecording}
      </DropdownMenuItem>
    </>
  );
}

/** Props of {@link RetranscribeDialogSlot}. */
export interface RetranscribeDialogSlotProps {
  meetingId: string;
  recording: LegacyRecording | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

/** The re-transcribe dialog, while the meeting has a recording to re-transcribe. */
export function RetranscribeDialogSlot({
  meetingId,
  recording,
  open,
  onOpenChange,
}: RetranscribeDialogSlotProps) {
  if (recording === null) return null;
  return (
    <RetranscribeDialog
      open={open}
      onOpenChange={onOpenChange}
      meetingId={meetingId}
      meetingFolderPath={recording.folderPath}
      onComplete={() => {
        void recording.refetchTranscripts();
      }}
    />
  );
}
