/**
 * The bridge between the inherited meeting session and the meeting page's
 * Transcript slot.
 *
 * The legacy session (`meeting-session.tsx`, `meeting-panels.tsx`) still
 * loads the transcript pages and publishes the finished panel here. The
 * Notes tab left the bridge with NOT-02; when TRN-01 rebuilds the
 * transcript on typed queries, the bridge goes away.
 */
import { createContext, type ReactNode, useContext } from 'react';

/** What the legacy session has to show. */
export type LegacyMeetingPanels =
  | { status: 'loading' }
  | { status: 'error'; message: string }
  | {
      status: 'ready';
      transcript: ReactNode;
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

/** The legacy panel, for the Transcript slot. */
export function useLegacyMeetingPanels(): LegacyMeetingPanels {
  return useContext(PanelsContext);
}
