/**
 * The follow-up editor (NOT-07): To/Cc/Bcc pre-filled with the external
 * attendees, the subject, and the message with its `[gaps]` highlighted.
 * It sends through the user's mail app (`mailto:`) or the clipboard; both
 * save the edit first. Sending from Deino through Gmail or Outlook joins
 * them with INT-07 (§8 D12); until then it is not shown (UX-27). Closing
 * with changes saves them.
 */
import { useState } from 'react';
import { toast } from 'sonner';

import type { MeetingId } from '@/lib/ipc/generated/meetings';
import { Button } from '@/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/ui/dialog';
import { Copy, ICON_PROPS, Mail } from '@/ui/icons';
import { Input } from '@/ui/input';

import { followUpsCopy } from '../copy';
import { editOf, fieldsOf, isDirty, placeholderCount } from '../draft-text';
import { useCopyFollowUp, useOpenInMail, useSaveFollowUp } from '../queries';
import type { DraftFields, FollowUpDraft } from '../types';
import { BodyField } from './body-field';
import { RecipientFields } from './recipient-fields';

const copy = followUpsCopy.editor;

/** Props of {@link FollowUpEditor}. */
export interface FollowUpEditorProps {
  draft: FollowUpDraft;
  /** The editor closed (after saving any change). */
  onClose: () => void;
}

/** Save, Mail and Copy over the current fields, each with its toast. */
function useEditorActions(meeting: MeetingId, fields: DraftFields) {
  const save = useSaveFollowUp();
  const mail = useOpenInMail();
  const copyBody = useCopyFollowUp();
  const request = () => ({ meeting, edit: editOf(fields) });
  const failed = (title: string) => (error: Error) => {
    toast.error(title, { description: error.message });
  };
  return {
    busy: mail.isPending || copyBody.isPending,
    save: () => {
      save.mutate(request(), { onError: failed(followUpsCopy.failed.save) });
    },
    openInMail: () => {
      mail.mutate(request(), { onError: failed(followUpsCopy.failed.mail) });
    },
    copy: () => {
      copyBody.mutate(request(), {
        onSuccess: () => toast(copy.copied),
        onError: failed(followUpsCopy.failed.copy),
      });
    },
  };
}

/** How many gaps are left, next to a swatch of the highlight. */
function GapCount({ fields }: { fields: DraftFields }) {
  const count = placeholderCount(fields.subject) + placeholderCount(fields.body);
  return (
    <p className="flex items-center gap-2 text-xs text-fg-muted">
      <span aria-hidden className="size-2.5 rounded-sm bg-highlight" />
      {copy.gaps(count)}
    </p>
  );
}

/** The editor's actions. */
function Footer({ actions }: { actions: ReturnType<typeof useEditorActions> }) {
  return (
    <DialogFooter className="flex-wrap items-center">
      <Button variant="secondary" disabled={actions.busy} onClick={actions.copy}>
        <Copy {...ICON_PROPS} />
        {copy.copy}
      </Button>
      <Button disabled={actions.busy} onClick={actions.openInMail}>
        <Mail {...ICON_PROPS} />
        {copy.openInMail}
      </Button>
    </DialogFooter>
  );
}

/** The recipients, subject and message, and how many gaps are left. */
function DraftForm({
  fields,
  onChange,
}: {
  fields: DraftFields;
  onChange: (key: keyof DraftFields, value: string) => void;
}) {
  return (
    <>
      <RecipientFields fields={fields} onChange={onChange} />
      <Input
        aria-label={copy.subject}
        value={fields.subject}
        className="font-medium"
        onChange={(event) => {
          onChange('subject', event.target.value);
        }}
      />
      <BodyField
        id="follow-up-body"
        label={copy.body}
        value={fields.body}
        onChange={(body) => {
          onChange('body', body);
        }}
      />
      <GapCount fields={fields} />
    </>
  );
}

/** The editor, open while mounted. */
export function FollowUpEditor({ draft, onClose }: FollowUpEditorProps) {
  const [fields, setFields] = useState(() => fieldsOf(draft));
  const actions = useEditorActions(draft.meetingId, fields);
  const set = (key: keyof DraftFields, value: string) => {
    setFields((current) => ({ ...current, [key]: value }));
  };
  const onOpenChange = (open: boolean) => {
    if (open) return;
    if (isDirty(fields, draft)) actions.save();
    onClose();
  };
  return (
    <Dialog open onOpenChange={onOpenChange}>
      <DialogContent className="max-w-[640px]">
        <DialogHeader>
          <DialogTitle>{copy.title}</DialogTitle>
          <DialogDescription>{copy.description}</DialogDescription>
        </DialogHeader>
        <DraftForm fields={fields} onChange={set} />
        <Footer actions={actions} />
      </DialogContent>
    </Dialog>
  );
}
