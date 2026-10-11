/**
 * The home screen, a viewer of the live recorder (FND-05b, CAP-02): the
 * notes of the meeting being recorded, its live transcript, and the
 * controls. Rust creates the meeting when recording starts, saves both the
 * notes (autosave) and the transcript as they happen, and hands the meeting
 * to the pipeline on stop, when its page opens (`RecordingSync`).
 */
import type { MeetingId } from '@/features/meetings';
import { Notepad, notepadCopy } from '@/features/notepad';
import {
  LiveTranscript,
  RecordingControls,
  recordingCopy,
  useRecordingState,
} from '@/features/recording';
import { EmptyState } from '@/ui/empty-state';
import { Mic } from '@/ui/icons';

/** The notes and transcript of the meeting being recorded. */
function LiveMeeting({ meetingId }: { meetingId: MeetingId }) {
  return (
    <>
      <section
        aria-label={notepadCopy.liveHeading}
        className="flex min-h-0 min-w-0 flex-1 flex-col overflow-y-auto"
      >
        <div className="mx-auto flex w-full max-w-[720px] flex-1 flex-col px-6 pb-24">
          <Notepad meetingId={meetingId} />
        </div>
      </section>
      <aside
        aria-label={notepadCopy.transcriptHeading}
        className="flex max-h-[40%] min-h-0 shrink-0 flex-col border-t border-border pb-20 lg:max-h-none lg:w-[360px] lg:border-t-0 lg:border-l"
      >
        <LiveTranscript meetingId={meetingId} />
      </aside>
    </>
  );
}

/** The record screen: idle, or the live meeting; the controls float below. */
export function RecordScreen() {
  const { meetingId } = useRecordingState();
  return (
    <div className="relative flex h-full flex-col bg-bg">
      <div className="flex min-h-0 flex-1 flex-col lg:flex-row">
        {meetingId === null ? (
          <EmptyState
            icon={Mic}
            title={recordingCopy.idleTitle}
            description={recordingCopy.idleBody}
          />
        ) : (
          <LiveMeeting key={meetingId} meetingId={meetingId} />
        )}
      </div>
      <div className="absolute inset-x-0 bottom-6 z-(--z-chatbar) flex justify-center px-6">
        <RecordingControls />
      </div>
    </div>
  );
}
