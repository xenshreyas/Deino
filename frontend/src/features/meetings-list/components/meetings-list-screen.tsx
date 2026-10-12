/**
 * The Meetings page (ORG-07, ui.md §4.4): saved views as tabs, filters that
 * persist, the list grouped by day and paged by cursor, bulk actions and
 * merge, and a link to the Trash. It waits for the settings, which keep the
 * tab and filter the user left.
 */
import { Link } from '@tanstack/react-router';

import { useSettings } from '@/features/settings';
import { buttonVariants } from '@/ui/button';
import { ICON_PROPS, Trash2 } from '@/ui/icons';
import { Kbd } from '@/ui/kbd';
import { Page, PageHeader } from '@/ui/page-header';
import { Skeleton } from '@/ui/skeleton';

import { copy } from '../copy';
import { ALL_MEETINGS, type MeetingsListPrefs, type TagId } from '../types';
import { ListPage } from './list-page';

/** Props of {@link MeetingsListScreen}; the route wires in the shell's actions. */
export interface MeetingsListScreenProps {
  onStartRecording: () => void;
  shortcuts: { newNote: string; record: string };
  /** Open filtered to these tags (a link from the tags palette) instead of the kept filter. */
  tags: readonly TagId[];
}

/** The page as first opened: "All meetings", unfiltered. */
const FRESH: MeetingsListPrefs = { view: null, filter: ALL_MEETINGS };

/** "Trash" and "New note". */
function HeaderActions({ newNote }: { newNote: string }) {
  return (
    <span className="flex items-center gap-2">
      <Link to="/meetings/trash" className={buttonVariants({ variant: 'ghost' })}>
        <Trash2 {...ICON_PROPS} />
        {copy.trash.nav}
      </Link>
      <Link to="/" className={buttonVariants({ variant: 'secondary' })}>
        {copy.newNote}
        <Kbd className="ml-1">{newNote}</Kbd>
      </Link>
    </span>
  );
}

/** The page. */
export function MeetingsListScreen({ onStartRecording, shortcuts, tags }: MeetingsListScreenProps) {
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
        actions={<HeaderActions newNote={shortcuts.newNote} />}
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
