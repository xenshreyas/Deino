/**
 * The find and replace confirmation (TRN-02): a popover from "Replace all"
 * that counts first, from Rust, exactly what the replacement will change
 * ("12 matches in 9 lines", and across which meetings), then applies that
 * plan on "Replace 12 matches". Nothing changes until the user confirms.
 */
import { toast } from 'sonner';

import { Button } from '@/ui/button';
import { Skeleton } from '@/ui/skeleton';

import { transcriptCopy } from '../copy';
import type { ReplacePreview, ReplaceRequest } from '../types';
import { useReplace, useReplacePreview } from './replace-queries';

/** The meetings listed before "and N more meetings". */
const MAX_LISTED = 5;

/** Props of {@link ReplaceConfirm}. */
export interface ReplaceConfirmProps {
  request: ReplaceRequest;
  replacement: string;
  /** Called after a replacement was applied, or on Cancel. */
  onClose: () => void;
}

/** "12 matches in 9 lines", and the meetings for a search across all of them. */
function Summary({ preview, all }: { preview: ReplacePreview; all: boolean }) {
  const listed = preview.meetings.slice(0, MAX_LISTED);
  const more = preview.meetings.length - listed.length;
  return (
    <div className="flex flex-col gap-2">
      <p className="text-sm font-semibold text-fg">
        {transcriptCopy.previewSummary(preview.matches, preview.segments)}
        {all ? ` ${transcriptCopy.previewMeetings(preview.meetings.length)}` : null}
      </p>
      {all ? (
        <ul className="flex flex-col gap-1 text-xs text-fg-muted">
          {listed.map((meeting) => (
            <li key={meeting.meetingId} className="flex justify-between gap-3">
              <span className="truncate">{meeting.title}</span>
              <span className="shrink-0 tabular-nums">{meeting.matches}</span>
            </li>
          ))}
          {more > 0 ? <li>{transcriptCopy.previewMore(more)}</li> : null}
        </ul>
      ) : null}
    </div>
  );
}

/** The counts, or their loading shape, failure or absence. */
function PreviewBody({ request }: { request: ReplaceRequest }) {
  const preview = useReplacePreview(request, true);
  if (preview.isPending) {
    return (
      <div role="status" aria-label={transcriptCopy.previewLoading} className="flex flex-col gap-2">
        <Skeleton className="h-4 w-40" />
        <Skeleton className="h-3 w-28" />
      </div>
    );
  }
  if (preview.isError) return <p className="text-sm text-danger">{transcriptCopy.previewFailed}</p>;
  if (preview.data.matches === 0) {
    return <p className="text-sm text-fg-muted">{transcriptCopy.previewNone}</p>;
  }
  return <Summary preview={preview.data} all={request.scope.kind === 'all'} />;
}

/** The popover's content: the counts, Cancel and Replace. */
export function ReplaceConfirm({ request, replacement, onClose }: ReplaceConfirmProps) {
  const preview = useReplacePreview(request, true);
  const replace = useReplace();
  const matches = preview.data?.matches ?? 0;
  const apply = () => {
    replace.mutate(
      { request, replacement },
      {
        onSuccess: (outcome) => {
          toast.success(transcriptCopy.replaced(outcome.matches));
          onClose();
        },
        onError: () => {
          toast.error(transcriptCopy.replaceFailed);
        },
      },
    );
  };
  return (
    <div className="flex flex-col gap-3">
      <PreviewBody request={request} />
      <div className="flex justify-end gap-2">
        <Button variant="ghost" size="sm" onClick={onClose}>
          {transcriptCopy.cancel}
        </Button>
        <Button size="sm" disabled={matches === 0 || replace.isPending} onClick={apply}>
          {transcriptCopy.replaceConfirm(matches)}
        </Button>
      </div>
    </div>
  );
}
