/**
 * A meeting's notepad as a whole window, without the app's chrome: what the
 * record panel's "Open notes" (CAP-10) opens. A 44px drag region stands in
 * for the title bar; the document keeps the 720px reading column.
 */
import type { MeetingId } from '@/features/meetings';

import { Notepad } from './notepad';

/** Props of {@link NotepadWindow}. */
export interface NotepadWindowProps {
  meetingId: MeetingId;
}

/** The stand-alone notepad. */
export function NotepadWindow({ meetingId }: NotepadWindowProps) {
  return (
    <div className="flex h-screen w-screen flex-col bg-bg text-fg">
      <div data-tauri-drag-region className="h-11 shrink-0" />
      <div className="mx-auto flex min-h-0 w-full max-w-[720px] flex-1 flex-col px-6">
        <Notepad meetingId={meetingId} />
      </div>
    </div>
  );
}
