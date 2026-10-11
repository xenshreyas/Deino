/**
 * The transcript's search bar (ui.md Transcript bubble, Search): a 36px
 * field at the top of the transcript with the speaker chips, the "3 of 27"
 * counter and the previous and next buttons; then the speaker filter and
 * the transcript's tools. ⌘F focuses it, Enter and ⇧Enter move between
 * hits, Escape clears it and Backspace in an empty field drops a chip.
 */
import { type KeyboardEvent, type ReactNode, type RefObject, useRef } from 'react';

import { useMountEffect } from '@/hooks/useMountEffect';
import { cn } from '@/ui/cn';
import { ChevronDown, ChevronUp, ICON_PROPS, Search } from '@/ui/icons';
import { IconButton } from '@/ui/icon-button';
import { Kbd } from '@/ui/kbd';

import { transcriptCopy } from '../copy';
import type { TranscriptSearch } from '../hooks/use-transcript-search';
import { isActive } from '../model/search-query';
import type { SpeakerView } from '../types';
import { SpeakerChips, SpeakerFilterMenu } from './speaker-filter';

/** Props of {@link SearchBar}. */
export interface SearchBarProps {
  search: TranscriptSearch;
  speakers: readonly SpeakerView[];
  /** Tools after the filter (Copy transcript, inherited actions). */
  tools?: ReactNode;
}

/** Focuses the field on ⌘F while the transcript is on screen. */
function useFindShortcut(input: RefObject<HTMLInputElement>): void {
  useMountEffect(() => {
    const listener = (event: globalThis.KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || event.key.toLowerCase() !== 'f') return;
      event.preventDefault();
      input.current?.focus();
      input.current?.select();
    };
    window.addEventListener('keydown', listener);
    return () => {
      window.removeEventListener('keydown', listener);
    };
  });
}

/** The field's keys. */
function onFieldKey(event: KeyboardEvent<HTMLInputElement>, search: TranscriptSearch): void {
  if (event.key === 'Enter') {
    event.preventDefault();
    if (event.shiftKey) search.previous();
    else search.next();
  } else if (event.key === 'Escape') {
    if (isActive(search.query)) search.clear();
    else event.currentTarget.blur();
  } else if (event.key === 'Backspace' && event.currentTarget.value === '') {
    search.popSpeaker();
  }
}

/** "3 of 27" and the previous and next buttons, while searching. */
function HitNavigation({ search }: { search: TranscriptSearch }) {
  const total = search.result.hits.length;
  const label =
    search.current === undefined
      ? transcriptCopy.noHits
      : transcriptCopy.hitCounter(search.current.ordinal + 1, total);
  return (
    <>
      <span
        role="status"
        className="shrink-0 px-1 text-xs whitespace-nowrap text-fg-muted tabular-nums"
      >
        {label}
      </span>
      <IconButton
        icon={ChevronUp}
        label={transcriptCopy.previousHit}
        shortcut={transcriptCopy.previousShortcut}
        disabled={total === 0}
        onClick={search.previous}
        className="size-6"
      />
      <IconButton
        icon={ChevronDown}
        label={transcriptCopy.nextHit}
        shortcut={transcriptCopy.nextShortcut}
        disabled={total === 0}
        onClick={search.next}
        className="size-6"
      />
    </>
  );
}

/** The field's frame: hairline at rest, the accent ring while focused. */
const FIELD_CLASS = cn(
  'flex h-9 min-w-0 flex-1 items-center gap-1.5 rounded-md border border-border bg-surface pr-1 pl-2.5',
  'transition-[border-color,box-shadow] duration-(--dur-fast) ease-out hover:border-border-strong',
  'focus-within:border-accent focus-within:shadow-[0_0_0_3px_color-mix(in_srgb,var(--accent)_20%,transparent)]',
);

/** The text input, its chips, and the counter or the ⌘F hint. */
function SearchField({ search, chosen }: { search: TranscriptSearch; chosen: SpeakerView[] }) {
  const input = useRef<HTMLInputElement>(null);
  useFindShortcut(input);
  return (
    <div role="search" className={FIELD_CLASS}>
      <Search {...ICON_PROPS} className="shrink-0 text-fg-subtle" />
      <SpeakerChips chosen={chosen} onRemove={search.toggleSpeaker} />
      <input
        ref={input}
        type="search"
        value={search.query.text}
        placeholder={chosen.length === 0 ? transcriptCopy.searchPlaceholder : ''}
        aria-label={transcriptCopy.searchLabel}
        aria-keyshortcuts="Meta+F"
        onChange={(event) => {
          search.setText(event.target.value);
        }}
        onKeyDown={(event) => {
          onFieldKey(event, search);
        }}
        className="h-full min-w-16 flex-1 bg-transparent text-sm text-fg outline-none placeholder:text-fg-subtle [&::-webkit-search-cancel-button]:hidden"
      />
      {isActive(search.query) ? (
        <HitNavigation search={search} />
      ) : (
        <Kbd className="mr-1">{transcriptCopy.searchShortcut}</Kbd>
      )}
    </div>
  );
}

/** The bar. */
export function SearchBar({ search, speakers, tools }: SearchBarProps) {
  const chosen = speakers.filter((speaker) => search.query.speakers.includes(speaker.key));
  return (
    <div className="flex shrink-0 items-center gap-1 pt-3 pb-2">
      <SearchField search={search} chosen={chosen} />
      <SpeakerFilterMenu
        speakers={speakers}
        chosen={search.query.speakers}
        onToggle={search.toggleSpeaker}
      />
      {tools}
    </div>
  );
}
