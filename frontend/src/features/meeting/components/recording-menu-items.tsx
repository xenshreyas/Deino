/**
 * The ⋯ menu's recording actions (UX-05): re-transcribe the recording
 * (`pipeline|retranscribe`: the engine in Settings, echo removed, then every
 * later step) and show it in Finder, which needs the recording folder the
 * inherited session publishes (`legacy/panels-context.tsx`).
 *
 * Every re-transcribe action lives here, one item each, so another engine
 * (for example "Re-transcribe with ElevenLabs") slots in next to
 * "Re-transcribe…".
 */
import { CloudTranscriptMenuItems } from '@/features/cloud-transcription';
import type { MeetingId } from '@/features/meetings';
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
  meetingId: MeetingId;
  recording: LegacyRecording | null;
  onRetranscribe: () => void;
}

/**
 * "Re-transcribe…", CAP-14's "Re-transcribe with ElevenLabs" (and "Restore
 * local transcript") when they apply, and "Show recording in Finder".
 */
export function RecordingMenuItems({
  meetingId,
  recording,
  onRetranscribe,
}: RecordingMenuItemsProps) {
  const missing = recording === null;
  return (
    <>
      <DropdownMenuItem onSelect={onRetranscribe}>
        <RotateCw {...ICON_PROPS} />
        {meetingCopy.retranscribe}
      </DropdownMenuItem>
      <CloudTranscriptMenuItems meetingId={meetingId} />
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
