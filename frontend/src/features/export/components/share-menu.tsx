import { useState } from 'react';

import { Button } from '@/ui/button';
import {
  Copy,
  FileDown,
  FileText,
  type Icon,
  ICON_PROPS,
  Mail,
  ScrollText,
  Share,
} from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/ui/menu';

import { exportCopy } from '../copy';
import { useShareActions } from '../hooks/use-share-actions';
import type { MeetingId } from '../types';
import { ExportDialog } from './export-dialog';

/** One menu action. */
interface ShareItem {
  icon: Icon;
  label: string;
  run: () => void;
}

function Item({ item }: { item: ShareItem }) {
  const ItemIcon = item.icon;
  return (
    <DropdownMenuItem onSelect={item.run}>
      <ItemIcon {...ICON_PROPS} />
      {item.label}
    </DropdownMenuItem>
  );
}

/** The menu's groups: export, the copies, email. */
function useShareGroups(meetingId: MeetingId, openExport: () => void): ShareItem[][] {
  const share = useShareActions(meetingId);
  return [
    [{ icon: FileDown, label: exportCopy.exportAs, run: openExport }],
    [
      {
        icon: Copy,
        label: exportCopy.copyRich,
        run: () => {
          share.copy('rich_text');
        },
      },
      {
        icon: FileText,
        label: exportCopy.copyMarkdown,
        run: () => {
          share.copy('markdown');
        },
      },
      {
        icon: ScrollText,
        label: exportCopy.copyTranscript,
        run: () => {
          share.copy('transcript');
        },
      },
    ],
    [{ icon: Mail, label: exportCopy.email, run: share.email }],
  ];
}

/**
 * The meeting's Share menu (ui.md §3 Meeting header): export to a file,
 * copy as rich text, Markdown or the transcript, or email the notes.
 * Everything stays on the Mac until the user sends it.
 */
export function ShareMenu({ meetingId }: { meetingId: MeetingId }) {
  const [exporting, setExporting] = useState(false);
  const groups = useShareGroups(meetingId, () => {
    setExporting(true);
  });
  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="secondary" size="sm" aria-label={exportCopy.shareLabel}>
            <Share {...ICON_PROPS} />
            {exportCopy.share}
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          {groups.map((group, index) => (
            <div key={group[0]?.label ?? index}>
              {index > 0 ? <DropdownMenuSeparator /> : null}
              {group.map((item) => (
                <Item key={item.label} item={item} />
              ))}
            </div>
          ))}
        </DropdownMenuContent>
      </DropdownMenu>
      <ExportDialog meetingId={meetingId} open={exporting} onOpenChange={setExporting} />
    </>
  );
}
