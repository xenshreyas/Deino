import { toast } from 'sonner';

import { exportCopy } from '../copy';
import { useCopyMeeting, useEmailMeeting } from '../queries';
import type { CopyKind, MeetingId } from '../types';

/** The confirmation each copy shows. */
const COPIED: Record<CopyKind, string> = {
  rich_text: exportCopy.copiedRich,
  markdown: exportCopy.copiedMarkdown,
  transcript: exportCopy.copiedTranscript,
};

/** An error's user-safe message (`IpcError` carries Rust's). */
export function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/**
 * Copy and email actions for a meeting. A copy is confirmed with a toast;
 * Mail opening is its own confirmation (UX-32), so only a failure toasts.
 * Used by the Share menu and the inherited panels' copy buttons.
 */
export function useShareActions(meetingId: MeetingId) {
  const copy = useCopyMeeting(meetingId);
  const email = useEmailMeeting(meetingId);
  return {
    copy: (kind: CopyKind) => {
      copy.mutate(kind, {
        onSuccess: () => toast.success(COPIED[kind]),
        onError: (error) => toast.error(messageOf(error)),
      });
    },
    email: () => {
      email.mutate(undefined, {
        onError: (error) => toast.error(messageOf(error)),
      });
    },
  };
}
