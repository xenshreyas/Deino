/**
 * The Notes tab's Enhanced view (NOT-02b): the status row (version
 * history, the quiet "Saved", Edit with AI, copy, and the page's Enhanced /
 * My notes switch), the edit bar while it is open (NOT-06), then the notes, or the state they are in: loading, being
 * written, not written yet, or failed. The page's `footer` (insight
 * answers, screen images) follows the notes, inside their scroll.
 *
 * While notes are written they stream in line by line (streamed notes):
 * the status row says what the run is doing and offers Stop, and the
 * finished lines replace the skeleton, or the old notes (dimmed until the
 * first new line arrives), until the saved version takes their place.
 */
import { type ReactNode, useState } from 'react';

import type { MeetingId } from '@/features/meetings';
import { SavedIndicator, type SaveStatus } from '@/features/notepad';
import { Alert, AlertDescription } from '@/ui/alert';
import { Button } from '@/ui/button';

import { notesCopy } from '../copy';
import { EditBar, EditWithAiButton } from '../edit';
import { type EnhanceAction, useEnhanceAction } from '../hooks/use-enhance-action';
import { type LiveNotes, shownDraft, useLiveNotes } from '../hooks/use-live-notes';
import { useEnhancedNotes, useIsEnhancing, useNoteVersions, useRestoreVersion } from '../queries';
import type { EnhancedNotesView, NotesDraft } from '../types';
import { CopyMenu } from './copy-menu';
import { DraftNotes } from './draft-notes';
import { NoNotesYet, NotesFailure, NotesSkeleton, WritingNotes } from './enhanced-states';
import { NotesDocument } from './notes-document';
import { RunNote, RunStatus, StopButton, useLockedHint } from './run-status';
import { VersionMenu } from './version-menu';

/** Props of {@link EnhancedNotes}. */
export interface EnhancedNotesProps {
  meetingId: MeetingId;
  /** Controls for the status row's right side (the view switch). */
  toolbar?: ReactNode;
  /** What follows the notes, scrolling with them. */
  footer?: ReactNode;
}

/** The Enhanced view. */
export function EnhancedNotes({ meetingId, toolbar, footer }: EnhancedNotesProps) {
  const [version, setVersion] = useState<number | null>(null);
  const [status, setStatus] = useState<SaveStatus>('idle');
  const notes = useEnhancedNotes(meetingId, version);
  const busy = useIsEnhancing(meetingId);
  const live = useLiveNotes(meetingId);
  const locked = useLockedHint();
  const view = notes.data ?? null;
  const draft = shownDraft(live, view);
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <StatusRow
        meetingId={meetingId}
        view={view}
        draft={draft}
        live={live}
        busy={busy}
        hint={draft === null ? null : locked.hint}
        toolbar={toolbar}
        onPick={setVersion}
        status={status}
      />
      {view?.isLatest === true && !busy ? (
        <EditBar meetingId={meetingId} version={view.version.version} />
      ) : null}
      {draft !== null && draft.lines > 0 ? (
        <DraftNotes draft={draft} footer={footer} onLocked={locked.flash} />
      ) : (
        <NotesBody
          meetingId={meetingId}
          query={notes}
          footer={footer}
          live={draft !== null}
          onVersion={setVersion}
          onStatus={setStatus}
        />
      )}
    </div>
  );
}

/** The row above the notes: version, "Saved", or what a run is doing; actions on the right. */
function StatusRow(props: {
  meetingId: MeetingId;
  view: EnhancedNotesView | null;
  draft: NotesDraft | null;
  live: LiveNotes | null;
  busy: boolean;
  hint: ReactNode;
  toolbar: ReactNode;
  onPick: (version: number | null) => void;
  status: SaveStatus;
}) {
  const { meetingId, view, draft } = props;
  const shown = view?.version.version ?? null;
  return (
    <div className="flex h-9 shrink-0 items-center justify-between gap-3">
      <div className="flex min-w-0 items-center gap-2">
        {draft !== null ? <RunStatus draft={draft} /> : null}
        {draft === null && shown !== null ? (
          <span className="shrink-0">
            <VersionMenu meetingId={meetingId} shown={shown} onPick={props.onPick} />
          </span>
        ) : null}
        {draft === null ? <SavedIndicator status={props.status} /> : null}
        {draft === null ? (
          <RunNote meetingId={meetingId} live={props.live} version={shown} />
        ) : null}
      </div>
      <div className="flex items-center gap-1">
        {draft !== null ? <StopButton meetingId={meetingId} draft={draft} /> : null}
        {draft === null && view?.isLatest === true ? (
          <EditWithAiButton disabled={props.busy} />
        ) : null}
        {draft === null && view !== null ? <CopyMenu meetingId={meetingId} /> : null}
        {props.hint}
        {props.toolbar}
      </div>
    </div>
  );
}

/** No notes yet: being written, or the Enhance offer; then the page's footer. */
function NoNotes(props: {
  busy: boolean;
  live: boolean;
  action: EnhanceAction;
  footer: ReactNode;
}) {
  const { action } = props;
  return (
    <div className="min-h-0 flex-1 overflow-y-auto">
      {props.busy || props.live ? (
        <WritingNotes quiet={props.live} />
      ) : (
        <NoNotesYet onEnhance={action.run} blocked={action.blocked} error={action.error} />
      )}
      {props.footer}
    </div>
  );
}

/** Below the status row: the notes or their state. */
function NotesBody({
  meetingId,
  query,
  footer,
  live,
  onVersion,
  onStatus,
}: {
  meetingId: MeetingId;
  query: ReturnType<typeof useEnhancedNotes>;
  footer: ReactNode;
  /** A run is showing its status in the row above. */
  live: boolean;
  onVersion: (version: number | null) => void;
  onStatus: (status: SaveStatus) => void;
}) {
  const busy = useIsEnhancing(meetingId) || live;
  const action = useEnhanceAction(meetingId);
  if (query.isPending) return <NotesSkeleton />;
  if (query.isError) return <LoadFailed onRetry={() => void query.refetch()} />;
  if (query.data === null) {
    return <NoNotes busy={busy} live={live} action={action} footer={footer} />;
  }
  return (
    <LoadedNotes
      meetingId={meetingId}
      view={query.data}
      busy={busy}
      live={live}
      action={action}
      footer={footer}
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
  live: boolean;
  action: EnhanceAction;
  footer: ReactNode;
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
        quiet={props.live}
        footer={props.footer}
        onRegenerate={action.blocked === null ? action.run : null}
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
