/**
 * The import dialog's steps: choose, ready, converting, done.
 * Presentational: the dialog owns the data and the mutations.
 */
import { useNavigate } from '@tanstack/react-router';
import { useId } from 'react';

import { MeetingEventChip } from '@/features/event-links';
import { Button } from '@/ui/button';
import { DialogFooter } from '@/ui/dialog';
import { AudioLines, CircleCheck, ICON_PROPS, Upload, Video } from '@/ui/icons';
import { Input } from '@/ui/input';
import { Skeleton } from '@/ui/skeleton';

import { importCopy as copy } from '../copy';
import type { ImportCandidate, ImportedMeeting } from '../types';

/** An inline, human-readable error line. */
function ErrorLine({ error }: { error: Error | null }) {
  if (error === null) return null;
  return (
    <p role="alert" className="text-sm text-danger">
      {error.message}
    </p>
  );
}

/** The file, with an audio or video icon. */
function FileLine({ candidate }: { candidate: ImportCandidate }) {
  const Icon = candidate.kind === 'video' ? Video : AudioLines;
  return (
    <div className="flex min-w-0 items-center gap-2 rounded-md bg-bg-subtle px-3 py-2 text-sm">
      <Icon {...ICON_PROPS} className="shrink-0 text-fg-muted" />
      <span className="truncate text-fg">{candidate.fileName}</span>
      <span className="ml-auto shrink-0 text-xs text-fg-muted">
        {candidate.kind === 'video' ? copy.video : copy.audio}
      </span>
    </div>
  );
}

/** Nothing chosen yet: one button, and the drop hint. */
export function ChooseStep(props: {
  checking: boolean;
  error: Error | null;
  onChoose: () => void;
}) {
  if (props.checking) return <Skeleton className="h-9 w-full" />;
  return (
    <div className="flex flex-col items-start gap-3">
      <Button variant="secondary" onClick={props.onChoose}>
        <Upload {...ICON_PROPS} />
        {copy.chooseFile}
      </Button>
      <p className="text-xs text-fg-muted">{copy.chooseHint}</p>
      <ErrorLine error={props.error} />
    </div>
  );
}

/** Props of {@link ReadyStep}. */
export interface ReadyStepProps {
  candidate: ImportCandidate;
  title: string;
  error: Error | null;
  onTitle: (title: string) => void;
  onImport: () => void;
  onChooseAnother: () => void;
}

/** A file is chosen: its name, an optional title and Import. */
export function ReadyStep({ candidate, title, error, ...actions }: ReadyStepProps) {
  const id = useId();
  return (
    <form
      className="flex flex-col gap-4"
      onSubmit={(event) => {
        event.preventDefault();
        actions.onImport();
      }}
    >
      <FileLine candidate={candidate} />
      <div className="flex flex-col gap-1.5">
        <label htmlFor={id} className="text-sm text-fg">
          {copy.titleLabel}
        </label>
        <Input
          id={id}
          value={title}
          placeholder={copy.titlePlaceholder}
          onChange={(event) => {
            actions.onTitle(event.target.value);
          }}
        />
      </div>
      <ErrorLine error={error} />
      <DialogFooter>
        <Button type="button" variant="ghost" onClick={actions.onChooseAnother}>
          {copy.chooseAnother}
        </Button>
        <Button type="submit">{copy.import}</Button>
      </DialogFooter>
    </form>
  );
}

/** Converting: the file and a calm progress line. */
export function ImportingStep({ candidate }: { candidate: ImportCandidate }) {
  return (
    <div className="flex flex-col gap-3" aria-busy="true">
      <FileLine candidate={candidate} />
      <Skeleton className="h-1 w-full" />
      <p className="text-sm text-fg">{copy.importing}</p>
      <p className="text-xs text-fg-muted">{copy.importingHint}</p>
    </div>
  );
}

/** Imported: the meeting, the event chip to link it, and a way in. */
export function DoneStep({ meeting, onClose }: { meeting: ImportedMeeting; onClose: () => void }) {
  const navigate = useNavigate();
  const open = () => {
    onClose();
    void navigate({ to: '/meetings/$meetingId', params: { meetingId: meeting.meetingId } });
  };
  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-start gap-2">
        <CircleCheck {...ICON_PROPS} className="mt-0.5 shrink-0 text-accent" />
        <div className="min-w-0">
          <p className="truncate text-sm font-semibold text-fg">{meeting.title}</p>
          <p className="text-xs text-fg-muted">
            {copy.minutes(meeting.duration)} · {copy.importedHint}
          </p>
        </div>
      </div>
      <div className="flex flex-col items-start gap-2">
        <p className="text-xs text-fg-muted">{copy.linkHint}</p>
        <MeetingEventChip meetingId={meeting.meetingId} />
      </div>
      <DialogFooter>
        <Button variant="ghost" onClick={onClose}>
          {copy.done}
        </Button>
        <Button onClick={open}>{copy.openMeeting}</Button>
      </DialogFooter>
    </div>
  );
}
