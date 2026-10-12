/**
 * The meeting's "View draft" banner (NOT-07): a quiet line at the top of
 * the Notes tab while a suggested follow-up is ready. "View draft" opens
 * the editor; dismissing asks once, because it is permanent.
 * {@link MeetingFollowUp} is the container; {@link FollowUpBanner} is
 * presentational.
 */
import { useState } from 'react';
import { toast } from 'sonner';

import type { MeetingId } from '@/lib/ipc/generated/meetings';
import { Button } from '@/ui/button';
import { ICON_PROPS, Mail, X } from '@/ui/icons';
import { IconButton } from '@/ui/icon-button';

import { followUpsCopy } from '../copy';
import { useDismissFollowUp, useMeetingFollowUp } from '../queries';
import { FollowUpEditor } from './follow-up-editor';

const copy = followUpsCopy.banner;

/** Props of {@link FollowUpBanner}. */
export interface FollowUpBannerProps {
  onView: () => void;
  onDismiss: () => void;
}

/** The inline "for good?" question that replaces the actions. */
function ConfirmDismiss({ onKeep, onDismiss }: { onKeep: () => void; onDismiss: () => void }) {
  return (
    <div className="flex items-center gap-1">
      <span className="text-xs text-fg-muted">{copy.confirm}</span>
      <Button variant="ghost" size="sm" onClick={onKeep}>
        {copy.keep}
      </Button>
      <Button variant="danger" size="sm" onClick={onDismiss}>
        {copy.confirmDismiss}
      </Button>
    </div>
  );
}

/** "Follow-up email ready", with View draft and dismiss. */
export function FollowUpBanner({ onView, onDismiss }: FollowUpBannerProps) {
  const [confirming, setConfirming] = useState(false);
  return (
    <section
      aria-label={copy.title}
      className="flex min-h-12 flex-wrap items-center gap-3 rounded-lg border border-border bg-bg-subtle px-3 py-2"
    >
      <Mail {...ICON_PROPS} className="shrink-0 text-fg-muted" />
      <div className="min-w-0 flex-1">
        <p className="text-sm font-medium text-fg">{copy.title}</p>
        <p className="truncate text-xs text-fg-muted">{copy.body}</p>
      </div>
      {confirming ? (
        <ConfirmDismiss
          onKeep={() => {
            setConfirming(false);
          }}
          onDismiss={onDismiss}
        />
      ) : (
        <div className="flex items-center gap-1">
          <Button variant="secondary" size="sm" onClick={onView}>
            {copy.view}
          </Button>
          <IconButton
            icon={X}
            label={copy.dismiss}
            onClick={() => {
              setConfirming(true);
            }}
          />
        </div>
      )}
    </section>
  );
}

/** The banner and its editor, while the meeting has a ready draft. */
export function MeetingFollowUp({ meetingId }: { meetingId: MeetingId }) {
  const followUp = useMeetingFollowUp(meetingId);
  const dismiss = useDismissFollowUp();
  const [editing, setEditing] = useState(false);
  const draft = followUp.data;
  if (!draft) return null;
  return (
    <div className="pt-4">
      <FollowUpBanner
        onView={() => {
          setEditing(true);
        }}
        onDismiss={() => {
          dismiss.mutate(meetingId, {
            onSuccess: () => toast(followUpsCopy.dismissed),
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
    </div>
  );
}
