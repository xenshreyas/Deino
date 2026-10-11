/**
 * The version history menu (Granola §3.3): every generation of the notes,
 * newest first, with when it was written, its template and whether the
 * user edited it. Choosing one shows it read-only; the newest is the
 * editable one.
 */
import { format } from 'date-fns';

import type { MeetingId } from '@/features/meetings';
import { cn } from '@/ui/cn';
import { Check, ChevronDown, Clock, ICON_PROPS } from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuTrigger,
} from '@/ui/menu';

import { notesCopy } from '../copy';
import { useNoteVersions } from '../queries';
import type { VersionInfo } from '../types';

/** Props of {@link VersionMenu}. */
export interface VersionMenuProps {
  meetingId: MeetingId;
  /** The version shown. */
  shown: number;
  /** Shows a version; `null` for the newest. */
  onPick: (version: number | null) => void;
}

/** `Oct 7, 15:20`. */
function when(iso: string): string {
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? '' : format(date, 'MMM d, HH:mm');
}

/** The menu; nothing until there are two versions to choose between. */
export function VersionMenu({ meetingId, shown, onPick }: VersionMenuProps) {
  const versions = useNoteVersions(meetingId).data ?? [];
  if (versions.length < 2) return null;
  const newest = versions[0]?.version;
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        aria-label={notesCopy.versionsLabel}
        className="flex h-6 items-center gap-1 rounded-sm px-1.5 text-xs text-fg-muted transition-colors duration-(--dur-fast) hover:bg-bg-muted hover:text-fg focus-visible:outline-2 focus-visible:outline-focus-ring"
      >
        <Clock {...ICON_PROPS} size={14} />
        {notesCopy.version(shown)}
        <ChevronDown {...ICON_PROPS} size={14} />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="w-64">
        <DropdownMenuLabel>{notesCopy.versionsLabel}</DropdownMenuLabel>
        {versions.map((version) => (
          <VersionItem
            key={version.version}
            version={version}
            current={version.version === shown}
            onSelect={() => {
              onPick(version.version === newest ? null : version.version);
            }}
          />
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

function VersionItem({
  version,
  current,
  onSelect,
}: {
  version: VersionInfo;
  current: boolean;
  onSelect: () => void;
}) {
  const detail = [version.templateName, version.isUserEdited ? notesCopy.edited : null]
    .filter((part) => part !== null)
    .join(' · ');
  return (
    <DropdownMenuItem onSelect={onSelect} className="h-auto py-1.5">
      <Check {...ICON_PROPS} className={cn('mt-0.5 self-start', !current && 'invisible')} />
      <span className="flex min-w-0 flex-col">
        <span className="text-sm text-fg">
          {notesCopy.version(version.version)}
          <span className="ml-1.5 text-xs text-fg-muted tabular-nums">
            {when(version.createdAt)}
          </span>
        </span>
        {detail === '' ? null : <span className="truncate text-xs text-fg-muted">{detail}</span>}
      </span>
    </DropdownMenuItem>
  );
}
