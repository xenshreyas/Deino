/**
 * The meeting's follow-up email (NOT-07), while a suggested draft is ready:
 * its subject, who it goes to, "Open draft" for the editor, and "Dismiss for
 * good" (it says so, because it is permanent). The meeting page shows it in
 * its context rail on a wide window and in the Details panel otherwise.
 * {@link MeetingFollowUp} is the container; {@link FollowUpBlock} is
 * presentational.
 */
import { useState } from 'react';
import { toast } from 'sonner';

import type { MeetingId } from '@/lib/ipc/generated/meetings';
import { Button } from '@/ui/button';
import { PanelSection } from '@/ui/panel-section';

import { followUpsCopy } from '../copy';
import { useDismissFollowUp, useMeetingFollowUp } from '../queries';
import type { FollowUpDraft } from '../types';
import { FollowUpEditor } from './follow-up-editor';

const copy = followUpsCopy.block;

/** How many different people the draft goes to (To, Cc and Bcc). */
export function recipientCount(draft: FollowUpDraft): number {
  const { to, cc, bcc } = draft.recipients;
  return new Set([...to, ...cc, ...bcc].map((address) => address.toLowerCase())).size;
}

/** Props of {@link FollowUpBlock}. */
export interface FollowUpBlockProps {
  draft: FollowUpDraft;
  onOpen: () => void;
  onDismiss: () => void;
}

/** "Follow-up email": the subject, "Draft ready · to N people", Open draft and Dismiss. */
export function FollowUpBlock({ draft, onOpen, onDismiss }: FollowUpBlockProps) {
  const subject = draft.subject.trim();
  return (
    <PanelSection title={copy.title}>
      <div className="flex flex-col gap-1.5 rounded-lg border border-border px-3 py-2.5">
        <span className="text-[13px]/[18px] text-pretty text-fg">
          {subject === '' ? copy.noSubject : subject}
        </span>
        <span className="text-xs text-fg-subtle">{copy.ready(recipientCount(draft))}</span>
        <div className="mt-1 flex items-center gap-1">
          <Button size="sm" onClick={onOpen} title={copy.hint}>
            {copy.open}
          </Button>
          <Button size="sm" variant="ghost" onClick={onDismiss}>
            {copy.dismiss}
          </Button>
        </div>
      </div>
    </PanelSection>
  );
}

/** The block and its editor, while the meeting has a ready draft. */
export function MeetingFollowUp({ meetingId }: { meetingId: MeetingId }) {
  const followUp = useMeetingFollowUp(meetingId);
  const dismiss = useDismissFollowUp();
  const [editing, setEditing] = useState(false);
  const draft = followUp.data;
  if (!draft) return null;
  return (
    <>
      <FollowUpBlock
        draft={draft}
        onOpen={() => {
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
