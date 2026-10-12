/**
 * The Notes tab's Enhanced view (NOT-02b): the status row (version
 * history, the quiet "Saved", Edit with AI, copy, and the page's Enhanced /
 * My notes switch), the edit bar while it is open (NOT-06), then the notes, or the state they are in: loading, being
 * written, not written yet, or failed.
 */
import { type ReactNode, useState } from 'react';

import type { MeetingId } from '@/features/meetings';
import { SavedIndicator, type SaveStatus } from '@/features/notepad';
import { Alert, AlertDescription } from '@/ui/alert';
import { Button } from '@/ui/button';

import { notesCopy } from '../copy';
import { EditBar, EditWithAiButton } from '../edit';
import { type EnhanceAction, useEnhanceAction } from '../hooks/use-enhance-action';
import { useEnhancedNotes, useIsEnhancing, useNoteVersions, useRestoreVersion } from '../queries';
import type { EnhancedNotesView } from '../types';
import { CopyMenu } from './copy-menu';
import { NoNotesYet, NotesFailure, NotesSkeleton, WritingNotes } from './enhanced-states';
import { NotesDocument } from './notes-document';
import { VersionMenu } from './version-menu';

/** Props of {@link EnhancedNotes}. */
export interface EnhancedNotesProps {
  meetingId: MeetingId;
  /** Controls for the status row's right side (the view switch). */
  toolbar?: ReactNode;
}

/** The Enhanced view. */
export function EnhancedNotes({ meetingId, toolbar }: EnhancedNotesProps) {
  const [version, setVersion] = useState<number | null>(null);
  const [status, setStatus] = useState<SaveStatus>('idle');
  const notes = useEnhancedNotes(meetingId, version);
  const busy = useIsEnhancing(meetingId);
  const view = notes.data ?? null;
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex h-9 shrink-0 items-center justify-between gap-3">
        <div className="flex min-w-0 items-center gap-2">
          {view === null ? null : (
            <VersionMenu meetingId={meetingId} shown={view.version.version} onPick={setVersion} />
          )}
          <SavedIndicator status={status} />
        </div>
        <div className="flex items-center gap-1">
          {view?.isLatest === true ? <EditWithAiButton disabled={busy} /> : null}
          {view === null ? null : <CopyMenu meetingId={meetingId} />}
          {toolbar}
        </div>
      </div>
      {view?.isLatest === true && !busy ? (
        <EditBar meetingId={meetingId} version={view.version.version} />
      ) : null}
      <NotesBody meetingId={meetingId} query={notes} onVersion={setVersion} onStatus={setStatus} />
    </div>
  );
}

/** Below the status row: the notes or their state. */
function NotesBody({
  meetingId,
  query,
  onVersion,
  onStatus,
}: {
  meetingId: MeetingId;
  query: ReturnType<typeof useEnhancedNotes>;
  onVersion: (version: number | null) => void;
  onStatus: (status: SaveStatus) => void;
}) {
  const busy = useIsEnhancing(meetingId);
  const action = useEnhanceAction(meetingId);
  if (query.isPending) return <NotesSkeleton />;
  if (query.isError) return <LoadFailed onRetry={() => void query.refetch()} />;
  if (query.data === null) {
    return busy ? <WritingNotes /> : <NoNotesYet onEnhance={action.run} error={action.error} />;
  }
  return (
    <LoadedNotes
      meetingId={meetingId}
      view={query.data}
      busy={busy}
      action={action}
      onVersion={onVersion}
      onStatus={onStatus}
    />
  );
}

/** The notes, with what regenerating or restoring needs. */
function LoadedNotes(props: {
  meetingId: MeetingId;
  view: EnhancedNotesView;
  busy: boolean;
  action: EnhanceAction;
  onVersion: (version: number | null) => void;
  onStatus: (status: SaveStatus) => void;
}) {
  const { meetingId, action, onVersion } = props;
  const versions = useNoteVersions(meetingId).data?.length ?? 1;
  const restore = useRestoreVersion(meetingId);
  return (
    <>
      {action.error === null ? null : <NotesFailure error={action.error} className="mb-3" />}
      <NotesDocument
        meetingId={meetingId}
        view={props.view}
        versions={versions}
        busy={props.busy}
        onRegenerate={action.run}
        onRestore={() => {
          restore.mutate(props.view.version.version, {
            onSuccess: () => {
              onVersion(null);
            },
          });
        }}
        onNewest={() => {
          onVersion(null);
        }}
        onStatus={props.onStatus}
      />
    </>
  );
}

/** The notes could not be read. */
function LoadFailed({ onRetry }: { onRetry: () => void }) {
  return (
    <Alert variant="destructive" className="mt-4">
      <AlertDescription className="flex items-center justify-between gap-3">
        <span>{notesCopy.loadFailed}</span>
        <Button variant="ghost" size="sm" onClick={onRetry}>
          {notesCopy.tryAgain}
        </Button>
      </AlertDescription>
    </Alert>
  );
}
