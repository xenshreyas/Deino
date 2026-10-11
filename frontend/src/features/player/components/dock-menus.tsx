/**
 * The dock's quiet text menus: the speed (0.75–2×), the audio track (Mixed,
 * Me, Them; only when there is more than one) and Download (each file the
 * meeting has, saved through Rust's save dialog).
 */
import { toast } from 'sonner';

import { buttonVariants } from '@/ui/button';
import { cn } from '@/ui/cn';
import { Check, ChevronDown, Download, ICON_PROPS } from '@/ui/icons';
import { IconButton } from '@/ui/icon-button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuTrigger,
} from '@/ui/menu';
import { Hint } from '@/ui/tooltip';

import { downloadLabels, playerCopy, sourceLabels } from '../copy';
import { useDownload } from '../queries';
import type { AudioSource, MediaSource, MeetingId } from '../types';

/** The speeds offered, slowest first. */
export const RATES = [0.75, 1, 1.25, 1.5, 1.75, 2] as const;

/** A compact text trigger: 28 high, muted, tabular. */
const TEXT_TRIGGER = cn(
  buttonVariants({ variant: 'ghost', size: 'sm' }),
  'h-7 gap-1 px-2 text-xs font-semibold text-fg-muted tabular-nums',
);

/** One checkable item. */
function Choice({
  label,
  chosen,
  onSelect,
}: {
  label: string;
  chosen: boolean;
  onSelect: () => void;
}) {
  return (
    <DropdownMenuItem onSelect={onSelect}>
      <Check {...ICON_PROPS} className={chosen ? 'opacity-100' : 'opacity-0'} />
      <span className="tabular-nums">{label}</span>
    </DropdownMenuItem>
  );
}

/** "1×": the speed menu. */
export function SpeedMenu({ rate, onRate }: { rate: number; onRate: (rate: number) => void }) {
  return (
    <DropdownMenu>
      <Hint label={playerCopy.speed}>
        <DropdownMenuTrigger aria-label={playerCopy.speed} className={cn(TEXT_TRIGGER, 'w-12')}>
          {playerCopy.rate(rate)}
        </DropdownMenuTrigger>
      </Hint>
      <DropdownMenuContent align="end" className="min-w-[140px]">
        <DropdownMenuLabel>{playerCopy.speed}</DropdownMenuLabel>
        {RATES.map((option) => (
          <Choice
            key={option}
            label={playerCopy.rate(option)}
            chosen={option === rate}
            onSelect={() => {
              onRate(option);
            }}
          />
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** Props of {@link SourceMenu}. */
export interface SourceMenuProps {
  sources: readonly AudioSource[];
  source: AudioSource;
  onSource: (source: AudioSource) => void;
}

/** "Mixed ▾": the track menu; nothing when there is one track. */
export function SourceMenu({ sources, source, onSource }: SourceMenuProps) {
  if (sources.length < 2) return null;
  return (
    <DropdownMenu>
      <Hint label={playerCopy.track}>
        <DropdownMenuTrigger aria-label={playerCopy.track} className={TEXT_TRIGGER}>
          {sourceLabels[source]}
          <ChevronDown {...ICON_PROPS} size={12} />
        </DropdownMenuTrigger>
      </Hint>
      <DropdownMenuContent align="end" className="min-w-[160px]">
        <DropdownMenuLabel>{playerCopy.track}</DropdownMenuLabel>
        {sources.map((option) => (
          <Choice
            key={option}
            label={sourceLabels[option]}
            chosen={option === source}
            onSelect={() => {
              onSource(option);
            }}
          />
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** The download menu over every file the meeting has. */
export function DownloadMenu({
  meetingId,
  sources,
}: {
  meetingId: MeetingId;
  sources: readonly MediaSource[];
}) {
  const download = useDownload(meetingId);
  const save = (source: MediaSource) => {
    download.mutate(source, {
      onSuccess: (outcome) => {
        if (outcome === 'saved') toast.success(playerCopy.saved);
      },
      onError: () => {
        toast.error(playerCopy.saveFailed);
      },
    });
  };
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <IconButton icon={Download} label={playerCopy.download} disabled={download.isPending} />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuLabel>{playerCopy.download}</DropdownMenuLabel>
        {sources.map((source) => (
          <DropdownMenuItem
            key={source}
            onSelect={() => {
              save(source);
            }}
          >
            <Download {...ICON_PROPS} />
            {downloadLabels[source]}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
