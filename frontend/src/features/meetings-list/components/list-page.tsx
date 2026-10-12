/**
 * The Meetings page once the settings have loaded: view tabs, the filter
 * bar, the paged list, the bulk bar and its dialogs. It owns the tab and
 * filter (`useListState`, saved to the settings) and wires the selection
 * and the keyboard (`useListShortcuts`).
 */
import { useQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';

import { filterOptionsQuery } from '@/features/action-items';
import { ExportDialog } from '@/features/export';

import { fileMeetings, mergeMeetings, tagMeetings, trashSelection } from '../actions';
import { copy } from '../copy';
import { isAllMeetings } from '../filters';
import { useListShortcuts } from '../hooks/use-list-shortcuts';
import { useListState } from '../hooks/use-list-state';
import { usePageStore } from '../page-store';
import { useMeetingPages, useMeetingViews } from '../queries';
import { select } from '../selection';
import type { FilterOptions, MeetingId, MeetingsListPrefs } from '../types';
import { BulkBar } from './bulk-bar';
import { ConfirmDialog } from './confirm-dialog';
import { FilterBar } from './filter-bar';
import { ListBody } from './list-body';
import { ViewTabs } from './view-tabs';

/** Props of {@link ListPage}. */
export interface ListPageProps {
  /** The tab and filter the settings kept. */
  initial: MeetingsListPrefs;
  onStartRecording: () => void;
  recordShortcut: string;
}

const NO_OPTIONS: FilterOptions = { people: [], companies: [], tags: [], folders: [] };

/** The page's state, queries and handlers. */
function usePage(initial: MeetingsListPrefs) {
  const views = useMeetingViews();
  const state = useListState(initial, views.data);
  useListShortcuts(initial.filter);
  const pages = useMeetingPages(state.filter);
  const options = useQuery(filterOptionsQuery()).data ?? NO_OPTIONS;
  const page = usePageStore();
  const order = pages.data?.pages.flatMap((p) => p.meetings.map((meeting) => meeting.id)) ?? [];
  const onSelect = (id: MeetingId, range: boolean) => {
    page.setSelection(select(page.selection, { id, range }, order));
  };
  return { views: views.data ?? [], state, pages, options, page, onSelect };
}

/** The merge confirmation; on success, opens the merged meeting. */
function MergeConfirm({ ids, onClose }: { ids: readonly MeetingId[]; onClose: () => void }) {
  const navigate = useNavigate();
  const merge = async () => {
    const outcome = await mergeMeetings([...ids]);
    if (outcome !== null)
      void navigate({ to: '/meetings/$meetingId', params: { meetingId: outcome.meetingId } });
  };
  return (
    <ConfirmDialog
      open={ids.length > 0}
      title={copy.merge.title(ids.length)}
      description={copy.merge.description}
      note={copy.merge.regenerate}
      confirm={copy.merge.confirm}
      cancel={copy.merge.cancel}
      tone="primary"
      onConfirm={merge}
      onClose={onClose}
    />
  );
}

/** The export dialog for one meeting, while `id` is set. */
function ExportOne({ id, onClose }: { id: MeetingId | null; onClose: () => void }) {
  if (id === null) return null;
  return (
    <ExportDialog
      meetingId={id}
      open
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    />
  );
}

/** The bulk bar over the selection, with its merge and export dialogs. */
function SelectionActions({ options }: { options: FilterOptions }) {
  const page = usePageStore();
  const [exporting, setExporting] = useState<MeetingId | null>(null);
  const ids = [...page.selection.ids];
  const only = ids.length === 1 ? ids[0] : undefined;
  return (
    <>
      <BulkBar
        count={ids.length}
        options={options}
        onTag={(tag) => void tagMeetings(ids, tag)}
        onFile={(folder) => void fileMeetings(ids, folder)}
        onExport={
          only === undefined
            ? null
            : () => {
                setExporting(only);
              }
        }
        onMerge={() => {
          page.askMerge(ids);
        }}
        onDelete={trashSelection}
        onClear={page.clearSelection}
      />
      <MergeConfirm
        ids={page.merging}
        onClose={() => {
          page.askMerge([]);
        }}
      />
      <ExportOne
        id={exporting}
        onClose={() => {
          setExporting(null);
        }}
      />
    </>
  );
}

/** The page. */
export function ListPage({ initial, onStartRecording, recordShortcut }: ListPageProps) {
  const { views, state, pages, options, page, onSelect } = usePage(initial);
  return (
    <>
      <ViewTabs views={views} state={state} />
      <FilterBar
        filter={state.filter}
        options={options}
        onChange={state.setFilter}
        menuOpen={page.menuOpen}
        onMenuOpenChange={page.setMenuOpen}
      />
      <ListBody
        pages={pages}
        filtering={!isAllMeetings(state.filter)}
        selected={page.selection.ids}
        onSelect={onSelect}
        onClearFilters={() => {
          state.choose({ kind: 'all' });
        }}
        onStartRecording={onStartRecording}
        recordShortcut={recordShortcut}
      />
      <SelectionActions options={options} />
    </>
  );
}
