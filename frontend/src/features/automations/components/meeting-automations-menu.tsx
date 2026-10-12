/**
 * A meeting's ⋯ → Automations menu (Circleback §10.1): every automation,
 * matching ones first, with its run status on this meeting and Run or
 * Rerun. Disabled and manual automations are listed too, for manual runs.
 */
import { useState } from 'react';

import { Button } from '@/ui/button';
import { Ellipsis, ICON_PROPS, Play, RotateCcw, Workflow } from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from '@/ui/menu';

import { runCopy } from '../copy';
import { useMeetingAutomations, useRunAutomation } from '../queries';
import type { MeetingAutomation, MeetingId } from '../types';
import { RunBadge } from './run-status';

/** Props of {@link MeetingAutomationsMenu}. */
export interface MeetingAutomationsMenuProps {
  meetingId: MeetingId;
}

/** The ⋯ button and its Automations submenu. */
export function MeetingAutomationsMenu({ meetingId }: MeetingAutomationsMenuProps) {
  const [open, setOpen] = useState(false);
  const menu = useMeetingAutomations(meetingId, open);
  return (
    <DropdownMenu open={open} onOpenChange={setOpen}>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" aria-label={runCopy.more}>
          <Ellipsis {...ICON_PROPS} />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
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
      </DropdownMenuContent>
    </DropdownMenu>
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
