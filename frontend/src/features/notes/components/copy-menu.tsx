/**
 * Copy the notes as Markdown or rich text, through EXP-01's copy (Rust
 * renders the stored enhanced notes, so the copy matches every export).
 */
import { toast } from 'sonner';

import { useCopyMeeting } from '@/features/export';
import type { MeetingId } from '@/features/meetings';
import { Copy, ICON_PROPS } from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/ui/menu';
import { Hint } from '@/ui/tooltip';

import { notesCopy } from '../copy';

/** The copy button and its two choices. */
export function CopyMenu({ meetingId }: { meetingId: MeetingId }) {
  const copy = useCopyMeeting(meetingId);
  const run = (kind: 'markdown' | 'rich_text') => {
    copy.mutate(kind, {
      onSuccess: () => toast.success(notesCopy.copied),
      onError: (error) => toast.error(notesCopy.copyFailed, { description: error.message }),
    });
  };
  return (
    <DropdownMenu>
      <Hint label={notesCopy.copyLabel}>
        <DropdownMenuTrigger
          aria-label={notesCopy.copyLabel}
          className="flex size-7 items-center justify-center rounded-sm text-fg-muted transition-colors duration-(--dur-fast) hover:bg-bg-muted hover:text-fg focus-visible:outline-2 focus-visible:outline-focus-ring"
        >
          <Copy {...ICON_PROPS} />
        </DropdownMenuTrigger>
      </Hint>
      <DropdownMenuContent align="end">
        <DropdownMenuItem
          onSelect={() => {
            run('markdown');
          }}
        >
          {notesCopy.copyMarkdown}
        </DropdownMenuItem>
        <DropdownMenuItem
          onSelect={() => {
            run('rich_text');
          }}
        >
          {notesCopy.copyRichText}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
