/**
 * The bridge between the inherited meeting session and the meeting page's
 * Transcript tab.
 *
 * The notes (NOT-01, NOT-02) and the transcript itself (TRN-01) read typed
 * queries now. What the legacy session still owns are the Transcript tab's
 * inherited tools, "open the recording folder" and "re-transcribe", which
 * need its untyped meeting metadata; it publishes them here and the
 * transcript view shows them beside its search. When CAP-08/CAP-11 port
 * re-transcription, the bridge goes away.
 */
import { createContext, type ReactNode, useContext } from 'react';

/** What the legacy session has to show. */
export type LegacyMeetingPanels =
  | { status: 'loading' }
  | { status: 'error'; message: string }
  | {
      status: 'ready';
      /** The Transcript tab's inherited tools: the recording folder and re-transcription. */
      transcriptTools: ReactNode;
    };

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

/** The legacy panels, for the Transcript tab's tools. */
export function useLegacyMeetingPanels(): LegacyMeetingPanels {
  return useContext(PanelsContext);
}
