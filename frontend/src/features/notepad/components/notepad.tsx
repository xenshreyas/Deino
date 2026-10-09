/**
 * "My notes" for a meeting: loads the notes once, then the editor owns the
 * document and autosaves it. Used on the meeting page and in the
 * stand-alone {@link NotepadWindow}.
 */
import type { ReactNode } from 'react';
import { useState } from 'react';

import type { MeetingId } from '@/features/meetings';
import { Alert, AlertDescription } from '@/ui/alert';
import { Button } from '@/ui/button';
import { CircleAlert, ICON_PROPS } from '@/ui/icons';
import { Skeleton, SkeletonText } from '@/ui/skeleton';

import { notepadCopy } from '../copy';
import { useNativeImageDrop } from '../hooks/use-native-image-drop';
import { useNoteEditor } from '../hooks/use-note-editor';
import { useMeetingNotes } from '../queries';
import { meetingTarget } from '../target';
import type { MeetingNotes } from '../types';
import { NoteSurface } from './note-surface';

/** Props of {@link Notepad}. */
export interface NotepadProps {
  meetingId: MeetingId;
  /** Controls for the status row's right side. */
  toolbar?: ReactNode;
}

/** The notes' loading shape: a few lines of a document. */
function NotesSkeleton({ toolbar }: { toolbar?: ReactNode }) {
  return (
    <div className="flex flex-col gap-4" role="status" aria-label="Loading">
      <div className="flex h-9 items-center justify-end">{toolbar}</div>
      <Skeleton className="h-3 w-1/3" />
      <SkeletonText lines={3} />
    </div>
  );
}

/** The editor over notes that have loaded. */
function LoadedNotepad({ notes, toolbar }: { notes: MeetingNotes; toolbar?: ReactNode }) {
  const [target] = useState(() => meetingTarget(notes));
  const blank = notes.updatedAt === null;
  const handle = useNoteEditor({ target, initialDoc: notes.doc, autoFocus: blank });
  useNativeImageDrop(handle.editor, target);
  return <NoteSurface handle={handle} toolbar={toolbar} />;
}

/** A meeting's notepad, or its loading or error state. */
export function Notepad({ meetingId, toolbar }: NotepadProps) {
  const notes = useMeetingNotes(meetingId);
  if (notes.isPending) return <NotesSkeleton toolbar={toolbar} />;
  if (notes.isError) {
    return (
      <Alert variant="destructive" className="mt-9">
        <CircleAlert {...ICON_PROPS} />
        <AlertDescription className="flex items-center justify-between gap-3">
          <span>{notepadCopy.loadError}</span>
          <Button variant="ghost" size="sm" onClick={() => void notes.refetch()}>
            {notepadCopy.retry}
          </Button>
        </AlertDescription>
      </Alert>
    );
  }
  return <LoadedNotepad key={meetingId} notes={notes.data} toolbar={toolbar} />;
}
