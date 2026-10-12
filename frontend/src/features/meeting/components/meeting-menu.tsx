/**
 * The meeting's ⋯ menu (UX-05): the rare actions, in one place.
 * Re-transcribe… and Show recording in Finder, then Run automation › and
 * Run insight ›, Processing details (every pipeline stage), and Move to
 * trash.
 */
import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';

import { MeetingAutomationsSubmenu } from '@/features/automations';
import { MeetingInsightsSubmenu } from '@/features/insights';
import type { MeetingId } from '@/features/meetings';
import { trashMeetings } from '@/features/meetings-list';
import { ProcessingDetailsDialog } from '@/features/pipeline';
import { Button } from '@/ui/button';
import { Ellipsis, ICON_PROPS, ListChecks, Trash2 } from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/ui/menu';

import { meetingCopy } from '../copy';
import type { LegacyRecording } from '../legacy/panels-context';
import { RecordingMenuItems, RetranscribeDialogSlot, useRecording } from './recording-menu-items';

/** The dialog a menu item opened, if any. */
type MenuDialog = 'retranscribe' | 'details' | null;

/** Moves the meeting to the trash (with Undo), then goes back to Meetings. */
function useTrash(meetingId: MeetingId) {
  const navigate = useNavigate();
  return () => {
    void trashMeetings([meetingId]).then(() => navigate({ to: '/meetings' }));
  };
}

/** The menu's items; `onOpen` opens the dialog an item stands for. */
function MenuItems(props: {
  meetingId: MeetingId;
  recording: LegacyRecording | null;
  onOpen: (dialog: MenuDialog) => void;
}) {
  const { meetingId, onOpen } = props;
  const trash = useTrash(meetingId);
  return (
    <DropdownMenuContent align="end" className="w-60">
      <RecordingMenuItems
        recording={props.recording}
        onRetranscribe={() => {
          onOpen('retranscribe');
        }}
      />
      <DropdownMenuSeparator />
      <MeetingAutomationsSubmenu meetingId={meetingId} />
      <MeetingInsightsSubmenu meetingId={meetingId} />
      <DropdownMenuItem
        onSelect={() => {
          onOpen('details');
        }}
      >
        <ListChecks {...ICON_PROPS} />
        {meetingCopy.processingDetails}
      </DropdownMenuItem>
      <DropdownMenuSeparator />
      <DropdownMenuItem destructive onSelect={trash}>
        <Trash2 {...ICON_PROPS} />
        {meetingCopy.moveToTrash}
      </DropdownMenuItem>
    </DropdownMenuContent>
  );
}

/** The ⋯ button, its menu and the dialogs it opens. */
export function MeetingMenu({ meetingId }: { meetingId: MeetingId }) {
  const [dialog, setDialog] = useState<MenuDialog>(null);
  const recording = useRecording();
  const closeIf = (open: boolean) => {
    if (!open) setDialog(null);
  };
  return (
    <>
      {/* Not modal, so a dialog opened from an item gets the pointer back. */}
      <DropdownMenu modal={false}>
        <DropdownMenuTrigger asChild>
          <Button variant="ghost" size="icon" aria-label={meetingCopy.more}>
            <Ellipsis {...ICON_PROPS} />
          </Button>
        </DropdownMenuTrigger>
        <MenuItems meetingId={meetingId} recording={recording} onOpen={setDialog} />
      </DropdownMenu>
      <ProcessingDetailsDialog
        meetingId={meetingId}
        open={dialog === 'details'}
        onOpenChange={closeIf}
      />
      <RetranscribeDialogSlot
        meetingId={meetingId}
        recording={recording}
        open={dialog === 'retranscribe'}
        onOpenChange={closeIf}
      />
    </>
  );
}
