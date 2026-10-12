/**
 * The bridge between the inherited meeting session and the meeting page.
 *
 * The notes (NOT-01, NOT-02) and the transcript itself (TRN-01) read typed
 * queries now. What the legacy session still owns is the meeting's
 * recording folder, which re-transcription and "Show recording in Finder"
 * need from its untyped metadata; it publishes them here and the meeting's
 * ⋯ menu (UX-05) offers them. When CAP-08/CAP-11 port re-transcription,
 * the bridge goes away.
 */
import { createContext, type ReactNode, useContext } from 'react';

/** The meeting's recording, as the legacy session knows it. */
export interface LegacyRecording {
  /** The recording folder on this Mac; `null` when the meeting has none. */
  folderPath: string | null;
  /** Shows the recording folder in Finder. */
  openFolder: () => Promise<void>;
  /** Reloads the transcript after a re-transcription. */
  refetchTranscripts: () => Promise<void>;
}

/** What the legacy session has to show. */
export type LegacyMeetingPanels =
  | { status: 'loading' }
  | { status: 'error'; message: string }
  | { status: 'ready'; recording: LegacyRecording };

const PanelsContext = createContext<LegacyMeetingPanels>({ status: 'loading' });

/** Publishes the session's panels to the slots below it. */
export function LegacyMeetingPanelsProvider({
  value,
  children,
}: {
  value: LegacyMeetingPanels;
  children: ReactNode;
}) {
  return <PanelsContext.Provider value={value}>{children}</PanelsContext.Provider>;
}

/** The legacy panels, for the ⋯ menu's recording actions. */
export function useLegacyMeetingPanels(): LegacyMeetingPanels {
  return useContext(PanelsContext);
}
