/**
 * A meeting's transcript, loaded: the container behind the meeting page's
 * Transcript tab and the record screen. It reads the saved transcript with
 * the live lines over it, and renders the loading shape, an inline error,
 * the resting state or the {@link TranscriptView}.
 */
import type { ReactNode } from 'react';

import { Alert, AlertDescription } from '@/ui/alert';
import { Button } from '@/ui/button';
import { EmptyState } from '@/ui/empty-state';
import { AudioLines, CircleAlert, ICON_PROPS, ScrollText } from '@/ui/icons';
import { Skeleton } from '@/ui/skeleton';

import { transcriptCopy } from '../copy';
import { useTranscript } from '../hooks/use-transcript';
import { TranscriptView, type TranscriptViewProps } from './transcript-view';

/** Props of {@link TranscriptPanel}. */
export type TranscriptPanelProps = Omit<TranscriptViewProps, 'rows' | 'speakers'>;

/** The loading shape: alternating turns. */
function TranscriptSkeleton() {
  return (
    <div className="flex flex-col gap-4 pt-16" role="status" aria-label={transcriptCopy.loading}>
      <Skeleton className="h-8 w-3/5 rounded-lg" />
      <Skeleton className="ml-auto h-8 w-1/2 rounded-lg" />
      <Skeleton className="h-12 w-2/3 rounded-lg" />
      <Skeleton className="ml-auto h-8 w-2/5 rounded-lg" />
    </div>
  );
}

/** The transcript could not be read: say so, with a retry. */
function TranscriptError({ retry }: { retry: () => void }) {
  return (
    <Alert variant="destructive" className="mt-6">
      <CircleAlert {...ICON_PROPS} />
      <AlertDescription className="flex items-center justify-between gap-3">
        <span>{transcriptCopy.loadError}</span>
        <Button variant="secondary" size="sm" onClick={retry}>
          {transcriptCopy.retry}
        </Button>
      </AlertDescription>
    </Alert>
  );
}

/** Nothing said yet: "Listening" while live, else "No transcript yet". */
function TranscriptEmpty({ live }: { live: boolean }) {
  return live ? (
    <EmptyState
      className="pt-[18vh]"
      icon={AudioLines}
      title={transcriptCopy.listeningTitle}
      description={transcriptCopy.listeningBody}
    />
  ) : (
    <EmptyState
      className="pt-[12vh]"
      icon={ScrollText}
      title={transcriptCopy.emptyTitle}
      description={transcriptCopy.emptyBody}
    />
  );
}

/** The transcript of `meetingId`, in whatever state it is. */
function TranscriptContent(props: TranscriptPanelProps): ReactNode {
  const state = useTranscript(props.meetingId);
  if (state.status === 'pending') return <TranscriptSkeleton />;
  if (state.status === 'error') return <TranscriptError retry={state.retry} />;
  if (state.rows.length === 0) return <TranscriptEmpty live={props.live} />;
  return <TranscriptView {...props} rows={state.rows} speakers={state.speakers} />;
}

/** The panel, remounted per meeting so live lines and search never carry over. */
export function TranscriptPanel(props: TranscriptPanelProps) {
  return <TranscriptContent key={props.meetingId} {...props} />;
}
