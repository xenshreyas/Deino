/**
 * A meeting's ⋯ → "Run automation" submenu (Circleback §10.1, UX-05):
 * every automation, matching ones first, with its run status on this
 * meeting and Run or Rerun. Disabled and manual automations are listed
 * too, for manual runs. The meeting page owns the ⋯ menu itself.
 */
import { ICON_PROPS, Play, RotateCcw, Workflow } from '@/ui/icons';
import {
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
} from '@/ui/menu';

import { runCopy } from '../copy';
import { useMeetingAutomations, useRunAutomation } from '../queries';
import type { MeetingAutomation, MeetingId } from '../types';
import { RunBadge } from './run-status';

/** Props of {@link MeetingAutomationsSubmenu}. */
export interface MeetingAutomationsSubmenuProps {
  meetingId: MeetingId;
}

/** "Run automation ›" and its list; it loads once the ⋯ menu opens. */
export function MeetingAutomationsSubmenu({ meetingId }: MeetingAutomationsSubmenuProps) {
  const menu = useMeetingAutomations(meetingId);
  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger>
        <Workflow {...ICON_PROPS} />
        {runCopy.menu}
      </DropdownMenuSubTrigger>
      <DropdownMenuSubContent className="w-80">
        {menu.data?.length === 0 ? (
          <DropdownMenuLabel>{runCopy.menuEmpty}</DropdownMenuLabel>
        ) : null}
        {(menu.data ?? []).map((entry) => (
          <AutomationItem key={entry.automation.id} meetingId={meetingId} entry={entry} />
        ))}
      </DropdownMenuSubContent>
    </DropdownMenuSub>
  );
}

/** One automation: name, status, and Run or Rerun. */
function AutomationItem({ meetingId, entry }: { meetingId: MeetingId; entry: MeetingAutomation }) {
  const run = useRunAutomation();
  const ran = entry.run !== null;
  return (
    <DropdownMenuItem
      disabled={run.isPending}
      onSelect={(event) => {
        event.preventDefault();
        run.mutate({ id: entry.automation.id, meetingIds: [meetingId] });
      }}
      className="h-auto py-1.5"
    >
      {ran ? <RotateCcw {...ICON_PROPS} /> : <Play {...ICON_PROPS} />}
      <span className="grid min-w-0 flex-1">
        <span className="truncate">{entry.automation.name}</span>
        <span className="text-xs text-fg-subtle">
          {ran ? runCopy.rerun : runCopy.run}
          {entry.matches ? '' : ` · ${runCopy.notMatching}`}
        </span>
      </span>
      <RunBadge run={entry.run} />
    </DropdownMenuItem>
  );
}
