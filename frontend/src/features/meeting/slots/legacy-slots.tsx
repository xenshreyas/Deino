/**
 * The Transcript tab, filled by the inherited session's panel
 * (`legacy/panels-context.tsx`) until TRN-01 rebuilds it.
 */
import { Link } from '@tanstack/react-router';
import type { ReactNode } from 'react';

import { Alert, AlertDescription } from '@/ui/alert';
import { CircleAlert, ICON_PROPS } from '@/ui/icons';
import { Skeleton } from '@/ui/skeleton';

import { meetingCopy } from '../copy';
import { type LegacyMeetingPanels, useLegacyMeetingPanels } from '../legacy/panels-context';

/** The transcript's loading shape: alternating turns. */
function TranscriptSkeleton() {
  return (
    <div className="flex flex-col gap-4 pt-6" role="status" aria-label="Loading">
      <Skeleton className="h-8 w-3/5 rounded-lg" />
      <Skeleton className="ml-auto h-8 w-1/2 rounded-lg" />
      <Skeleton className="h-12 w-2/3 rounded-lg" />
    </div>
  );
}

/** The session failed: say so inline, with a way back. */
function SessionError({ message }: { message: string }) {
  return (
    <Alert variant="destructive" className="mt-6">
      <CircleAlert {...ICON_PROPS} />
      <AlertDescription className="flex flex-col gap-1">
        <span>{meetingCopy.loadError}</span>
        <span className="text-fg-muted">{message}</span>
        <Link to="/meetings" className="text-danger underline underline-offset-4">
          {meetingCopy.backToMeetings}
        </Link>
      </AlertDescription>
    </Alert>
  );
}

/** Renders one published panel, or its loading or error state. */
function LegacyPanel({
  pick,
  skeleton,
}: {
  pick: (ready: Extract<LegacyMeetingPanels, { status: 'ready' }>) => ReactNode;
  skeleton: ReactNode;
}) {
  const panels = useLegacyMeetingPanels();
  if (panels.status === 'loading') return skeleton;
  if (panels.status === 'error') return <SessionError message={panels.message} />;
  return <div className="flex h-full min-h-0 flex-col">{pick(panels)}</div>;
}

/** Transcript: the paginated transcript with copy and retranscribe. */
export function TranscriptSlot() {
  return <LegacyPanel pick={(ready) => ready.transcript} skeleton={<TranscriptSkeleton />} />;
}
