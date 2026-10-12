/**
 * The list area's states: loading skeleton, an inline error, "no meetings
 * yet" with the record action, "no matches" with Clear filters, or the
 * grouped list.
 */
import { Alert, AlertDescription } from '@/ui/alert';
import { Button } from '@/ui/button';
import { EmptyState } from '@/ui/empty-state';
import { CircleAlert, ICON_PROPS, ListFilter, Mic, NotebookText } from '@/ui/icons';
import { Kbd } from '@/ui/kbd';
import { Skeleton } from '@/ui/skeleton';

import { copy } from '../copy';
import type { useMeetingPages } from '../queries';
import type { MeetingId } from '../types';
import { MeetingList } from './meeting-list';

/** Props of {@link ListBody}. */
export interface ListBodyProps {
  pages: ReturnType<typeof useMeetingPages>;
  /** True when the filter narrows the list. */
  filtering: boolean;
  selected: ReadonlySet<MeetingId>;
  onSelect: (id: MeetingId, range: boolean) => void;
  onClearFilters: () => void;
  onStartRecording: () => void;
  recordShortcut: string;
}

/** Five row-shaped placeholders while the list loads. */
function ListSkeleton() {
  return (
    <div role="status" aria-label="Loading" className="flex flex-col gap-1 pt-2">
      {Array.from({ length: 5 }, (_, index) => (
        <div key={index} className="flex h-10 items-center gap-3 px-3">
          <Skeleton className="h-2.5 flex-1" />
          <Skeleton className="size-5 rounded-full" />
          <Skeleton className="h-2.5 w-10" />
        </div>
      ))}
    </div>
  );
}

/** The error, with a retry. */
function LoadError({ onRetry }: { onRetry: () => void }) {
  return (
    <Alert variant="destructive">
      <CircleAlert {...ICON_PROPS} />
      <AlertDescription className="flex items-center justify-between gap-3">
        {copy.loadError}
        <Button variant="ghost" size="sm" onClick={onRetry}>
          {copy.retry}
        </Button>
      </AlertDescription>
    </Alert>
  );
}

/** No meeting at all, or none the filter matches. */
function Empty(
  props: Pick<
    ListBodyProps,
    'filtering' | 'onClearFilters' | 'onStartRecording' | 'recordShortcut'
  >,
) {
  if (props.filtering) {
    const action = (
      <Button variant="secondary" onClick={props.onClearFilters}>
        {copy.clearFilters}
      </Button>
    );
    return (
      <EmptyState
        className="pt-[10vh]"
        icon={ListFilter}
        title={copy.noMatchesTitle}
        description={copy.noMatchesDescription}
        action={action}
      />
    );
  }
  const action = (
    <Button variant="secondary" onClick={props.onStartRecording}>
      <Mic {...ICON_PROPS} />
      {copy.startRecording}
      <Kbd className="ml-1">{props.recordShortcut}</Kbd>
    </Button>
  );
  return (
    <EmptyState
      className="pt-[14vh]"
      icon={NotebookText}
      title={copy.emptyTitle}
      description={copy.emptyDescription}
      action={action}
    />
  );
}

/** The list area. */
export function ListBody(props: ListBodyProps) {
  const { pages } = props;
  if (pages.isPending) return <ListSkeleton />;
  if (pages.isError) return <LoadError onRetry={() => void pages.refetch()} />;
  const meetings = pages.data.pages.flatMap((page) => page.meetings);
  if (meetings.length === 0) return <Empty {...props} />;
  const more = pages.hasNextPage && !pages.isFetchingNextPage;
  return (
    <MeetingList
      meetings={meetings}
      selected={props.selected}
      onSelect={props.onSelect}
      onLoadMore={more ? () => void pages.fetchNextPage() : null}
    />
  );
}
