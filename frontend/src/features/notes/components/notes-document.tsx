/**
 * One version of the notes on the page: the regenerate banner (newest
 * only), the bar for an older version, the scrolling document and its
 * table of contents. While new notes are being written the old ones stay,
 * dimmed, under a quiet status line.
 */
import { type UIEvent, useRef, useState } from 'react';

import type { MeetingId } from '@/features/meetings';
import type { SaveStatus } from '@/features/notepad';
import { useShellLayout } from '@/features/shell';
import { Button } from '@/ui/button';
import { cn } from '@/ui/cn';
import { History, ICON_PROPS, Sparkles } from '@/ui/icons';

import { notesCopy } from '../copy';
import { EnhancedEditor } from '../lazy';
import type { EnhancedNotesView, TocEntry } from '../types';
import { NotesToc, tocEntries } from './notes-toc';
import { RegenerateBanner } from './regenerate-banner';

/** A heading counts as "in view" once it is this close to the top, in px. */
const HEADING_IN_VIEW = 48;

/** Props of {@link NotesDocument}. */
export interface NotesDocumentProps {
  meetingId: MeetingId;
  view: EnhancedNotesView;
  /** Total versions, for the older-version bar. */
  versions: number;
  busy: boolean;
  /** Writes the notes again; `null` when they cannot be (PRV-01). */
  onRegenerate: (() => void) | null;
  onRestore: () => void;
  onNewest: () => void;
  onStatus: (status: SaveStatus) => void;
}

/** The banner's action when the notes cannot be regenerated; it is hidden then. */
const NO_REGENERATE = (): void => undefined;

/** The headings of the rendered notes, in order. */
function headingsIn(scroller: HTMLElement): HTMLElement[] {
  return [...scroller.querySelectorAll<HTMLElement>('.enhanced-notes :is(h1, h2, h3)')];
}

/** The heading in view as the notes scroll, and jumping to one. */
function useContents() {
  const scroller = useRef<HTMLDivElement>(null);
  const [active, setActive] = useState(0);
  const onScroll = (event: UIEvent<HTMLDivElement>) => {
    const top = event.currentTarget.getBoundingClientRect().top;
    const passed = headingsIn(event.currentTarget).filter(
      (heading) => heading.getBoundingClientRect().top - top <= HEADING_IN_VIEW,
    );
    setActive(Math.max(0, passed.length - 1));
  };
  const jump = (entry: TocEntry) => {
    const root = scroller.current;
    if (root === null) return;
    headingsIn(root)[entry.index]?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  };
  return { scroller, active, onScroll, jump };
}

/** The notes and everything around them. */
export function NotesDocument(props: NotesDocumentProps) {
  const { view, busy } = props;
  const { scroller, active, onScroll, jump } = useContents();
  const [dismissed, setDismissed] = useState<number | null>(null);
  const rightPanelOpen = useShellLayout((state) => state.rightPanelOpen);
  const hidden = dismissed === view.version.version || busy || props.onRegenerate === null;
  const reasons = hidden ? [] : view.reasons;
  return (
    <div className="relative flex min-h-0 flex-1 flex-col">
      {busy ? <RewritingLine /> : null}
      <RegenerateBanner
        reasons={reasons}
        onRegenerate={props.onRegenerate ?? NO_REGENERATE}
        onDismiss={() => {
          setDismissed(view.version.version);
        }}
      />
      {view.isLatest ? null : <OlderVersionBar {...props} />}
      <div
        ref={scroller}
        onScroll={onScroll}
        className={cn(
          '-ml-9 min-h-0 flex-1 overflow-y-auto pb-[30vh] pl-9 transition-opacity duration-(--dur-base)',
          busy && 'pointer-events-none opacity-50',
        )}
      >
        <EnhancedEditor
          key={`${view.meetingId}:${String(view.version.version)}`}
          meetingId={props.meetingId}
          view={view}
          onStatus={props.onStatus}
        />
      </div>
      {rightPanelOpen ? null : (
        <NotesToc entries={tocEntries(view.doc)} active={active} onJump={jump} />
      )}
    </div>
  );
}

/** "Writing new notes…" above the dimmed old ones. */
function RewritingLine() {
  return (
    <p role="status" className="mb-3 flex items-center gap-2 text-sm text-ai-text">
      <Sparkles {...ICON_PROPS} className="animate-pulse text-accent" />
      {notesCopy.rewriting}
    </p>
  );
}

/** "Version 1 of 3", with Restore and Back to newest. */
function OlderVersionBar({ view, versions, onRestore, onNewest }: NotesDocumentProps) {
  return (
    <div className="mb-3 flex items-center gap-3 rounded-lg bg-bg-muted px-3 py-2 text-sm text-fg">
      <History {...ICON_PROPS} className="shrink-0 text-fg-muted" />
      <span className="min-w-0 flex-1">{notesCopy.viewingOld(view.version.version, versions)}</span>
      <Button variant="ghost" size="sm" onClick={onNewest}>
        {notesCopy.backToNewest}
      </Button>
      <Button variant="secondary" size="sm" onClick={onRestore}>
        {notesCopy.restore}
      </Button>
    </div>
  );
}
