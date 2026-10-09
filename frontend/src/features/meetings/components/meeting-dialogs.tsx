import { type FormEvent, useState } from 'react';
import { toast } from 'sonner';

import { Button } from '@/ui/button';
import { Label } from '@/ui/controls';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/ui/dialog';
import { Input } from '@/ui/input';

import { meetingsCopy } from '../copy';
import { useDeleteMeeting, useRenameMeeting } from '../queries';
import type { Meeting } from '../types';

/** Props of the meeting dialogs: the meeting, or `null` while closed. */
export interface MeetingDialogProps {
  meeting: Meeting | null;
  onClose: () => void;
}

/** The rename form; mounted per meeting so the field starts from its title. */
function RenameForm({ meeting, onClose }: { meeting: Meeting; onClose: () => void }) {
  const [title, setTitle] = useState(meeting.title);
  const rename = useRenameMeeting();
  const renamed = () => {
    toast.success(meetingsCopy.renamed);
    onClose();
  };
  const submit = (event: FormEvent) => {
    event.preventDefault();
    rename.mutate({ id: meeting.id, title: title.trim() }, { onSuccess: renamed });
  };
  return (
    <form onSubmit={submit} className="grid gap-4">
      <div className="grid gap-1.5">
        <Label htmlFor="meeting-title">{meetingsCopy.titleLabel}</Label>
        <Input
          id="meeting-title"
          value={title}
          onChange={(event) => {
            setTitle(event.target.value);
          }}
          aria-invalid={rename.isError}
          autoFocus
        />
        {rename.isError ? <p className="text-xs text-danger">{rename.error.message}</p> : null}
      </div>
      <DialogFooter>
        <Button variant="ghost" onClick={onClose}>
          {meetingsCopy.cancel}
        </Button>
        <Button type="submit" disabled={title.trim() === '' || rename.isPending}>
          {meetingsCopy.save}
        </Button>
      </DialogFooter>
    </form>
  );
}

/** Renames a meeting. */
export function RenameMeetingDialog({ meeting, onClose }: MeetingDialogProps) {
  return (
    <Dialog
      open={meeting !== null}
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <DialogContent aria-describedby={undefined}>
        <DialogHeader>
          <DialogTitle>{meetingsCopy.renameTitle}</DialogTitle>
        </DialogHeader>
        {meeting === null ? null : (
          <RenameForm key={meeting.id} meeting={meeting} onClose={onClose} />
        )}
      </DialogContent>
    </Dialog>
  );
}

/** Confirms and deletes a meeting (irreversible, so it asks). */
export function DeleteMeetingDialog({ meeting, onClose }: MeetingDialogProps) {
  const remove = useDeleteMeeting();
  const confirm = () => {
    if (meeting === null) return;
    remove.mutate(meeting.id, {
      onSuccess: () => {
        toast.success(meetingsCopy.deleted);
        onClose();
      },
      onError: (error) => {
        toast.error(meetingsCopy.deleteFailed, { description: error.message });
      },
    });
  };
  return (
    <Dialog
      open={meeting !== null}
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{meetingsCopy.deleteTitle}</DialogTitle>
          <DialogDescription>
            {meetingsCopy.deleteDescription(meeting?.title ?? '')}
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="ghost" onClick={onClose}>
            {meetingsCopy.cancel}
          </Button>
          <Button variant="danger-solid" onClick={confirm} disabled={remove.isPending}>
            {meetingsCopy.delete}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
