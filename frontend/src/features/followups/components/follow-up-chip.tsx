/**
 * The meeting header's "Follow-up email ready" chip (NOT-07, UX-07): shown
 * while a suggested follow-up is ready. Its menu opens the draft in the
 * editor, or dismisses it for good (the item says so, because it is
 * permanent). {@link MeetingFollowUp} is the container; {@link FollowUpChip}
 * is presentational.
 */
import { useState } from 'react';
import { toast } from 'sonner';

import type { MeetingId } from '@/lib/ipc/generated/meetings';
import { Chip } from '@/ui/chip';
import { ICON_PROPS, Mail, Pencil, X } from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/ui/menu';

import { followUpsCopy } from '../copy';
import { useDismissFollowUp, useMeetingFollowUp } from '../queries';
import { FollowUpEditor } from './follow-up-editor';

const copy = followUpsCopy.chip;

/** Props of {@link FollowUpChip}. */
export interface FollowUpChipProps {
  onView: () => void;
  onDismiss: () => void;
}

/** "Follow-up email ready", opening View draft and Dismiss for good. */
export function FollowUpChip({ onView, onDismiss }: FollowUpChipProps) {
  return (
    <DropdownMenu modal={false}>
      <DropdownMenuTrigger asChild>
        <Chip icon={Mail} title={copy.hint}>
          {copy.title}
        </Chip>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="w-56">
        <DropdownMenuItem onSelect={onView}>
          <Pencil {...ICON_PROPS} />
          {copy.view}
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem destructive onSelect={onDismiss}>
          <X {...ICON_PROPS} />
          {copy.dismiss}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** The chip and its editor, while the meeting has a ready draft. */
export function MeetingFollowUp({ meetingId }: { meetingId: MeetingId }) {
  const followUp = useMeetingFollowUp(meetingId);
  const dismiss = useDismissFollowUp();
  const [editing, setEditing] = useState(false);
  const draft = followUp.data;
  if (!draft) return null;
  return (
    <>
      <FollowUpChip
        onView={() => {
          setEditing(true);
        }}
        onDismiss={() => {
          dismiss.mutate(meetingId, {
            onError: (error) =>
              toast.error(followUpsCopy.failed.dismiss, { description: error.message }),
          });
        }}
      />
      {editing ? (
        <FollowUpEditor
          draft={draft}
          onClose={() => {
            setEditing(false);
          }}
        />
      ) : null}
    </>
  );
}
