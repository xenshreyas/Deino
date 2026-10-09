/**
 * The Notes and Transcript tabs, filled by the inherited session's panels
 * (see `legacy/panels-context.tsx`) until NOT-01 and TRN-01 rebuild them.
 * The Notes tab shows the meeting's template picker (NOT-04) above them.
 */
import { Link } from '@tanstack/react-router';
import type { ReactNode } from 'react';

import { MeetingTemplatePicker } from '@/features/templates';
import { Alert, AlertDescription } from '@/ui/alert';
import { CircleAlert, ICON_PROPS } from '@/ui/icons';
import { Skeleton, SkeletonText } from '@/ui/skeleton';

import { meetingCopy } from '../copy';
import { type LegacyMeetingPanels, useLegacyMeetingPanels } from '../legacy/panels-context';
import type { MeetingSlotProps } from '../types';

/** The notes' loading shape: a heading and a few bullet lines. */
function NotesSkeleton() {
  return (
    <div className="flex flex-col gap-4 pt-6">
      <Skeleton className="h-3 w-1/3" />
      <SkeletonText lines={4} />
    </div>
  );
}

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

/**
 * Notes: the meeting's template (NOT-04: which one, why, switch it) above
 * the AI summary editor, generate and regenerate.
 */
export function NotesSlot({ meetingId }: MeetingSlotProps) {
  const panels = useLegacyMeetingPanels();
  return (
    <div className="flex h-full min-h-0 flex-col">
      <MeetingTemplatePicker
        meetingId={meetingId}
        onRegenerate={panels.status === 'ready' ? panels.regenerateNotes : null}
      />
      <LegacyPanel pick={(ready) => ready.notes} skeleton={<NotesSkeleton />} />
    </div>
  );
}

/** Transcript: the paginated transcript with copy and retranscribe. */
export function TranscriptSlot() {
  return <LegacyPanel pick={(ready) => ready.transcript} skeleton={<TranscriptSkeleton />} />;
}
