/**
 * The replace row under the transcript's search bar (TRN-02): what to put
 * in place of the search field's text, "Match case", this meeting or all
 * meetings, and "Replace all", which opens the counted confirmation. The
 * text to find is the search field's (its `speaker:` chips do not narrow a
 * replacement: it changes every match in scope).
 */
import { type KeyboardEvent, useState } from 'react';

import { Button } from '@/ui/button';
import { cn } from '@/ui/cn';
import { CaseSensitive, ICON_PROPS, Replace } from '@/ui/icons';
import { Popover, PopoverAnchor, PopoverContent } from '@/ui/popover';
import { SegmentedControl } from '@/ui/segmented-control';
import { Hint } from '@/ui/tooltip';

import { transcriptCopy } from '../copy';
import type { MeetingId } from '../types';
import { ReplaceConfirm } from './replace-confirm';
import { type ReplaceReach, replaceRequest } from './replace-queries';

/** Props of {@link ReplaceBar}. */
export interface ReplaceBarProps {
  meetingId: MeetingId;
  /** The search field's text. */
  find: string;
}

const REACH = [
  { value: 'meeting', label: transcriptCopy.scopeMeeting },
  { value: 'all', label: transcriptCopy.scopeAll },
] as const;

/** "Aa": match case, a pressed toggle. */
function MatchCase({ on, onToggle }: { on: boolean; onToggle: () => void }) {
  return (
    <Hint label={transcriptCopy.matchCase} side="top">
      <button
        type="button"
        aria-label={transcriptCopy.matchCase}
        aria-pressed={on}
        onClick={onToggle}
        className={cn(
          'flex size-7 shrink-0 items-center justify-center rounded-sm transition-colors duration-(--dur-fast) ease-out focus-visible:outline-2 focus-visible:outline-focus-ring',
          on ? 'bg-accent-soft text-accent' : 'text-fg-muted hover:bg-bg-muted hover:text-fg',
        )}
      >
        <CaseSensitive {...ICON_PROPS} />
      </button>
    </Hint>
  );
}

/** The row. */
export function ReplaceBar({ meetingId, find }: ReplaceBarProps) {
  const [replacement, setReplacement] = useState('');
  const [reach, setReach] = useState<ReplaceReach>('meeting');
  const [matchCase, setMatchCase] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const ready = find.trim() !== '';
  const request = replaceRequest(meetingId, find, { reach, matchCase });
  const onKey = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'Enter' && ready) {
      event.preventDefault();
      setConfirming(true);
    }
  };
  return (
    <Popover open={confirming && ready} onOpenChange={setConfirming}>
      <div className="flex shrink-0 items-center gap-1 pb-2">
        <label className="flex h-9 min-w-0 flex-1 items-center gap-1.5 rounded-md border border-border bg-surface pr-1 pl-2.5 transition-[border-color,box-shadow] duration-(--dur-fast) ease-out focus-within:border-accent focus-within:shadow-[0_0_0_3px_color-mix(in_srgb,var(--accent)_20%,transparent)] hover:border-border-strong">
          <Replace {...ICON_PROPS} className="shrink-0 text-fg-subtle" />
          <input
            value={replacement}
            placeholder={transcriptCopy.replacePlaceholder}
            aria-label={transcriptCopy.replaceLabel}
            onChange={(event) => {
              setReplacement(event.target.value);
            }}
            onKeyDown={onKey}
            className="h-full min-w-16 flex-1 bg-transparent text-sm text-fg outline-none placeholder:text-fg-subtle"
          />
          <MatchCase on={matchCase} onToggle={() => setMatchCase((on) => !on)} />
        </label>
        <SegmentedControl label={transcriptCopy.replace} segments={REACH} value={reach} onValueChange={setReach} />
        <PopoverAnchor asChild>
          <Button
            variant="secondary"
            size="sm"
            disabled={!ready}
            title={ready ? undefined : transcriptCopy.replaceFindFirst}
            onClick={() => {
              setConfirming(true);
            }}
          >
            {transcriptCopy.replaceAll}
          </Button>
        </PopoverAnchor>
      </div>
      <PopoverContent align="end" className="w-80">
        {confirming && ready ? (
          <ReplaceConfirm
            request={request}
            replacement={replacement}
            onClose={() => {
              setConfirming(false);
            }}
          />
        ) : null}
      </PopoverContent>
    </Popover>
  );
}
