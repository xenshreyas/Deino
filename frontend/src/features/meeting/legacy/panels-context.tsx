/**
 * The bridge between the inherited meeting session and the meeting page's
 * slots.
 *
 * The legacy session (`meeting-session.tsx`, `meeting-panels.tsx`) still
 * loads the transcript and summary and owns their shared state (the custom
 * prompt, summary generation, polling). It publishes the two finished
 * panels here; the Notes and Transcript slots render them. When NOT-01 and
 * TRN-01 rebuild those tabs on typed queries, their slots stop reading this
 * context and the bridge goes away.
 */
import { createContext, type ReactNode, useContext } from 'react';

/** What the legacy session has to show. */
export type LegacyMeetingPanels =
  | { status: 'loading' }
  | { status: 'error'; message: string }
  | {
      status: 'ready';
      notes: ReactNode;
      transcript: ReactNode;
      /** Regenerates the notes with the meeting's template; `null` before there are notes. */
      regenerateNotes: (() => void) | null;
      /** Whether there is an AI summary; the Notes tab opens on "My notes" without one. */
      hasSummary: boolean;
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

/** The legacy panels, for the Notes and Transcript slots. */
export function useLegacyMeetingPanels(): LegacyMeetingPanels {
  return useContext(PanelsContext);
}
