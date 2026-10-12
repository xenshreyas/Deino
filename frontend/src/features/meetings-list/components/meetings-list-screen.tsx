/**
 * The Meetings page (ORG-07, ui.md §4.4): saved views as tabs, filters that
 * persist, the list grouped by day and paged by cursor, bulk actions and
 * merge, and in the header Tags, Trash and Import (UX-25, UX-26: they left
 * the sidebar). It waits for the settings, which keep the tab and filter the
 * user left.
 */
import { Link } from '@tanstack/react-router';

import { useSettings } from '@/features/settings';
import { Button, buttonVariants } from '@/ui/button';
import { ICON_PROPS, Tag, Trash2, Upload } from '@/ui/icons';
import { Kbd } from '@/ui/kbd';
import { Page, PageHeader } from '@/ui/page-header';
import { Skeleton } from '@/ui/skeleton';

import { copy } from '../copy';
import { ALL_MEETINGS, type MeetingsListPrefs, type TagId } from '../types';
import { ListPage } from './list-page';

/** Props of {@link MeetingsListScreen}; the route wires in the shell's actions. */
export interface MeetingsListScreenProps {
  onStartRecording: () => void;
  /** Opens the import dialog. */
  onImport: () => void;
  shortcuts: { newNote: string; record: string };
  /** Open filtered to these tags (a link from the tags palette) instead of the kept filter. */
  tags: readonly TagId[];
}

/** The page as first opened: "All meetings", unfiltered. */
const FRESH: MeetingsListPrefs = { view: null, filter: ALL_MEETINGS };

/** "Tags", "Trash", "Import" and "New note". */
function HeaderActions({ newNote, onImport }: { newNote: string; onImport: () => void }) {
  return (
    <span className="flex items-center gap-1">
      <Link to="/tags" className={buttonVariants({ variant: 'ghost' })}>
        <Tag {...ICON_PROPS} />
        {copy.tagsNav}
      </Link>
      <Link to="/meetings/trash" className={buttonVariants({ variant: 'ghost' })}>
        <Trash2 {...ICON_PROPS} />
        {copy.trash.nav}
      </Link>
      <Button variant="ghost" onClick={onImport}>
        <Upload {...ICON_PROPS} />
        {copy.importRecording}
      </Button>
      <Link to="/" className={buttonVariants({ variant: 'secondary', className: 'ml-1' })}>
        {copy.newNote}
        <Kbd className="ml-1">{newNote}</Kbd>
      </Link>
    </span>
  );
}

/** The page. */
export function MeetingsListScreen(props: MeetingsListScreenProps) {
  const { onStartRecording, onImport, shortcuts, tags } = props;
  const settings = useSettings();
  // Settings that cannot be read leave the page usable, starting from "All meetings".
  const kept = settings.data?.ui.meetingsList ?? (settings.isError ? FRESH : undefined);
  const initial =
    tags.length === 0 || kept === undefined
      ? kept
      : { view: null, filter: { ...ALL_MEETINGS, tags: [...tags] } };
  return (
    <Page>
      <PageHeader
        title={copy.title}
        className="pb-2"
        actions={<HeaderActions newNote={shortcuts.newNote} onImport={onImport} />}
      />
      {initial === undefined ? (
        <div role="status" aria-label="Loading" className="pt-4">
          <Skeleton className="h-8 w-64" />
        </div>
      ) : (
        <ListPage
          key={tags.join()}
          initial={initial}
          onStartRecording={onStartRecording}
          recordShortcut={shortcuts.record}
        />
      )}
    </Page>
  );
}
